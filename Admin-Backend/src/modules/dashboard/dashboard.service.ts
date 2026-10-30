import { Injectable } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { MoreThan, Repository } from 'typeorm';
import { InterviewSessionStatus } from '../../common/enums';
import { Candidate } from '../candidates/candidate.entity';
import { InterviewSession } from '../interview-sessions/interview-session.entity';
import { Resume } from '../resumes/resume.entity';
import { DesktopDevice } from '../desktop-integration/desktop-device.entity';

@Injectable()
export class DashboardService {
  constructor(
    @InjectRepository(InterviewSession) private readonly sessions: Repository<InterviewSession>,
    @InjectRepository(Resume) private readonly resumes: Repository<Resume>,
    @InjectRepository(Candidate) private readonly candidates: Repository<Candidate>,
    @InjectRepository(DesktopDevice) private readonly devices: Repository<DesktopDevice>,
  ) {}

  async summary(organizationId: string) {
    const [total, completed, upcoming, resumes, candidates, recentSessions, devices] =
      await Promise.all([
        this.sessions.count({ where: { organizationId } }),
        this.sessions.count({
          where: { organizationId, status: InterviewSessionStatus.COMPLETED },
        }),
        this.sessions.count({
          where: { organizationId, status: InterviewSessionStatus.SCHEDULED },
        }),
        this.resumes.count({ where: { organizationId } }),
        this.candidates.count({ where: { organizationId } }),
        this.sessions.find({
          where: { organizationId },
          relations: ['candidate'],
          order: { updatedAt: 'DESC' },
          take: 6,
        }),
        this.devices.find({
          where: { organizationId, lastSeenAt: MoreThan(new Date(Date.now() - 5 * 60 * 1000)) },
        }),
      ]);

    const trends = await this.sessions
      .createQueryBuilder('s')
      .select("DATE_TRUNC('day', s.createdAt)", 'day')
      .addSelect('COUNT(*)', 'count')
      .where('s.organizationId = :organizationId', { organizationId })
      .groupBy('day')
      .orderBy('day', 'ASC')
      .limit(14)
      .getRawMany();

    return {
      totals: { sessions: total, completed, upcoming, resumes, candidates },
      desktop: {
        connected: devices.length > 0,
        activeDevices: devices.length,
      },
      recentSessions,
      trends,
    };
  }
}
