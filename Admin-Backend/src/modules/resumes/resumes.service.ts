import {
  BadRequestException,
  Injectable,
  NotFoundException,
} from '@nestjs/common';
import { ConfigService } from '@nestjs/config';
import { InjectRepository } from '@nestjs/typeorm';
import { ILike, Repository } from 'typeorm';
import { randomUUID } from 'crypto';
import { FileStorageService } from '../file-storage/file-storage.service';
import { Resume } from './resume.entity';
import { ResumeVersion } from './resume-version.entity';

const ALLOWED = new Set([
  'application/pdf',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
]);

@Injectable()
export class ResumesService {
  constructor(
    @InjectRepository(Resume) private readonly resumes: Repository<Resume>,
    @InjectRepository(ResumeVersion) private readonly versions: Repository<ResumeVersion>,
    private readonly files: FileStorageService,
    private readonly config: ConfigService,
  ) {}

  list(organizationId: string, search?: string) {
    return this.resumes.find({
      where: search
        ? [
            { organizationId, originalFilename: ILike(`%${search}%`) },
          ]
        : { organizationId },
      relations: ['candidate'],
      order: { updatedAt: 'DESC' },
    });
  }

  async get(organizationId: string, id: string) {
    const resume = await this.resumes.findOne({
      where: { id, organizationId },
      relations: ['candidate', 'versions'],
    });
    if (!resume) throw new NotFoundException('Resume not found');
    return resume;
  }

  async upload(
    organizationId: string,
    userId: string,
    file: Express.Multer.File,
    candidateId?: string,
  ) {
    this.assertFile(file);
    const key = `org/${organizationId}/resumes/${randomUUID()}/${file.originalname}`;
    await this.files.put(key, file.buffer, file.mimetype);

    const extracted = this.extractMetadata(file);
    const resume = await this.resumes.save(
      this.resumes.create({
        organizationId,
        candidateId: candidateId || null,
        originalFilename: file.originalname,
        mimeType: file.mimetype,
        sizeBytes: file.size,
        storageKey: key,
        storageProvider: this.config.get('storage.provider'),
        extractedText: extracted.text,
        extractedJson: extracted.json,
        status: 'uploaded',
        currentVersion: 1,
      }),
    );

    await this.versions.save(
      this.versions.create({
        resumeId: resume.id,
        versionNumber: 1,
        storageKey: key,
        originalFilename: file.originalname,
        extractedText: extracted.text,
        extractedJson: extracted.json,
        createdById: userId,
      }),
    );

    return resume;
  }

  async addVersion(organizationId: string, id: string, userId: string, file: Express.Multer.File) {
    const resume = await this.get(organizationId, id);
    this.assertFile(file);
    const next = resume.currentVersion + 1;
    const key = `org/${organizationId}/resumes/${resume.id}/v${next}/${file.originalname}`;
    await this.files.put(key, file.buffer, file.mimetype);
    const extracted = this.extractMetadata(file);

    resume.currentVersion = next;
    resume.originalFilename = file.originalname;
    resume.mimeType = file.mimetype;
    resume.sizeBytes = file.size;
    resume.storageKey = key;
    resume.extractedText = extracted.text;
    resume.extractedJson = extracted.json;
    await this.resumes.save(resume);

    await this.versions.save(
      this.versions.create({
        resumeId: resume.id,
        versionNumber: next,
        storageKey: key,
        originalFilename: file.originalname,
        extractedText: extracted.text,
        extractedJson: extracted.json,
        createdById: userId,
      }),
    );
    return resume;
  }

  async download(organizationId: string, id: string) {
    const resume = await this.get(organizationId, id);
    const buffer = await this.files.get(resume.storageKey);
    return { buffer, resume };
  }

  async associateCandidate(organizationId: string, id: string, candidateId: string) {
    const resume = await this.get(organizationId, id);
    resume.candidateId = candidateId;
    return this.resumes.save(resume);
  }

  async remove(organizationId: string, id: string) {
    const resume = await this.get(organizationId, id);
    await this.files.delete(resume.storageKey);
    await this.resumes.remove(resume);
    return { success: true };
  }

  private assertFile(file?: Express.Multer.File) {
    if (!file) throw new BadRequestException('Resume file is required');
    const max = this.config.get<number>('uploads.maxBytes') ?? 10_485_760;
    if (file.size > max) throw new BadRequestException('File exceeds size limit');
    if (!ALLOWED.has(file.mimetype)) {
      throw new BadRequestException('Only PDF and DOCX resumes are allowed');
    }
    if (file.mimetype === 'application/pdf' && file.buffer.subarray(0, 4).toString() !== '%PDF') {
      throw new BadRequestException('Invalid PDF file');
    }
  }

  private extractMetadata(file: Express.Multer.File) {
    const base = file.originalname.replace(/\.[^.]+$/, '').replace(/[_-]+/g, ' ');
    const parts = base.split(' ').filter(Boolean);
    const json = {
      suggestedName: base,
      possibleFirstName: parts[0] ?? null,
      possibleLastName: parts.slice(1).join(' ') || null,
      sourceFilename: file.originalname,
      extraction: 'filename_heuristic',
      skills: [] as string[],
      experience: [] as string[],
    };
    return { text: `Resume uploaded: ${file.originalname}`, json };
  }
}
