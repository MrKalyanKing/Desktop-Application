import { Injectable, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { ILike, Repository } from 'typeorm';
import { Candidate } from './candidate.entity';

@Injectable()
export class CandidatesService {
  constructor(
    @InjectRepository(Candidate) private readonly candidates: Repository<Candidate>,
  ) {}

  list(organizationId: string, search?: string) {
    return this.candidates.find({
      where: search
        ? [
            { organizationId, firstName: ILike(`%${search}%`) },
            { organizationId, lastName: ILike(`%${search}%`) },
            { organizationId, email: ILike(`%${search}%`) },
          ]
        : { organizationId },
      order: { updatedAt: 'DESC' },
    });
  }

  async get(organizationId: string, id: string) {
    const candidate = await this.candidates.findOne({ where: { id, organizationId } });
    if (!candidate) throw new NotFoundException('Candidate not found');
    return candidate;
  }

  create(organizationId: string, data: Partial<Candidate>) {
    return this.candidates.save(this.candidates.create({ ...data, organizationId }));
  }

  async update(organizationId: string, id: string, data: Partial<Candidate>) {
    const candidate = await this.get(organizationId, id);
    Object.assign(candidate, data);
    return this.candidates.save(candidate);
  }

  async remove(organizationId: string, id: string) {
    const candidate = await this.get(organizationId, id);
    await this.candidates.remove(candidate);
    return { success: true };
  }
}
