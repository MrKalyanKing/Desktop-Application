import { Injectable, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import {
  DesktopSyncStatus,
  InterviewSessionStatus,
  InterviewType,
} from '../../common/enums';
import { InterviewSession } from './interview-session.entity';

export interface CreateSessionInput {
  title: string;
  jobTitle?: string;
  company?: string;
  interviewType?: InterviewType;
  candidateId?: string;
  resumeId?: string;
  scheduledAt?: string;
  notes?: string;
  consentCaptured?: boolean;
}

@Injectable()
export class InterviewSessionsService {
  constructor(
    @InjectRepository(InterviewSession)
    private readonly sessions: Repository<InterviewSession>,
  ) {}

  async list(
    organizationId: string,
    filters: { status?: InterviewSessionStatus; search?: string; type?: InterviewType; candidateId?: string },
  ) {
    const qb = this.sessions
      .createQueryBuilder('s')
      .leftJoinAndSelect('s.candidate', 'candidate')
      .leftJoinAndSelect('s.resume', 'resume')
      .where('s.organizationId = :organizationId', { organizationId })
      .orderBy('s.updatedAt', 'DESC');

    if (filters.status) qb.andWhere('s.status = :status', { status: filters.status });
    if (filters.type) qb.andWhere('s.interviewType = :type', { type: filters.type });
    if (filters.candidateId) qb.andWhere('s.candidateId = :candidateId', { candidateId: filters.candidateId });
    if (filters.search) {
      qb.andWhere('(s.title ILIKE :q OR s.jobTitle ILIKE :q OR s.company ILIKE :q)', {
        q: `%${filters.search}%`,
      });
    }
    return qb.getMany();
  }

  async get(organizationId: string, id: string) {
    const session = await this.sessions.findOne({
      where: { id, organizationId },
      relations: ['candidate', 'resume', 'createdBy'],
    });
    if (!session) throw new NotFoundException('Interview session not found');
    return session;
  }

  create(organizationId: string, userId: string, input: CreateSessionInput) {
    return this.sessions.save(
      this.sessions.create({
        organizationId,
        createdById: userId,
        title: input.title,
        jobTitle: input.jobTitle ?? null,
        company: input.company ?? null,
        interviewType: input.interviewType ?? InterviewType.GENERAL,
        candidateId: input.candidateId ?? null,
        resumeId: input.resumeId ?? null,
        scheduledAt: input.scheduledAt ? new Date(input.scheduledAt) : null,
        notes: input.notes ?? null,
        consentCaptured: Boolean(input.consentCaptured),
        status: input.scheduledAt
          ? InterviewSessionStatus.SCHEDULED
          : InterviewSessionStatus.DRAFT,
      }),
    );
  }

  async update(organizationId: string, id: string, input: Partial<CreateSessionInput>) {
    const session = await this.get(organizationId, id);
    if (input.title) session.title = input.title;
    if (input.jobTitle !== undefined) session.jobTitle = input.jobTitle ?? null;
    if (input.company !== undefined) session.company = input.company ?? null;
    if (input.interviewType) session.interviewType = input.interviewType;
    if (input.candidateId !== undefined) session.candidateId = input.candidateId ?? null;
    if (input.resumeId !== undefined) session.resumeId = input.resumeId ?? null;
    if (input.scheduledAt !== undefined) {
      session.scheduledAt = input.scheduledAt ? new Date(input.scheduledAt) : null;
    }
    if (input.notes !== undefined) session.notes = input.notes ?? null;
    if (input.consentCaptured !== undefined) session.consentCaptured = input.consentCaptured;
    return this.sessions.save(session);
  }

  async start(organizationId: string, id: string) {
    const session = await this.get(organizationId, id);
    session.status = InterviewSessionStatus.IN_PROGRESS;
    session.startedAt = session.startedAt ?? new Date();
    session.desktopSyncStatus = DesktopSyncStatus.SYNCING;
    return this.sessions.save(session);
  }

  async complete(organizationId: string, id: string) {
    const session = await this.get(organizationId, id);
    session.status = InterviewSessionStatus.COMPLETED;
    session.endedAt = new Date();
    session.desktopSyncStatus = DesktopSyncStatus.SYNCED;
    return this.sessions.save(session);
  }

  async archive(organizationId: string, id: string) {
    const session = await this.get(organizationId, id);
    session.status = InterviewSessionStatus.ARCHIVED;
    return this.sessions.save(session);
  }

  async remove(organizationId: string, id: string) {
    const session = await this.get(organizationId, id);
    await this.sessions.remove(session);
    return { success: true };
  }

  async setSyncStatus(organizationId: string, id: string, status: DesktopSyncStatus) {
    const session = await this.get(organizationId, id);
    session.desktopSyncStatus = status;
    return this.sessions.save(session);
  }

  async bumpTranscriptSequence(organizationId: string, id: string, sequence: number) {
    const session = await this.get(organizationId, id);
    session.lastTranscriptSequence = Math.max(session.lastTranscriptSequence, sequence);
    return this.sessions.save(session);
  }
}
