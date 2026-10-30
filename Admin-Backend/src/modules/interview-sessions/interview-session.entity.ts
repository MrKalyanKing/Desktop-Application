import {
  Column,
  CreateDateColumn,
  Entity,
  Index,
  JoinColumn,
  ManyToOne,
  PrimaryGeneratedColumn,
  UpdateDateColumn,
} from 'typeorm';
import {
  DesktopSyncStatus,
  InterviewSessionStatus,
  InterviewType,
} from '../../common/enums';
import { Organization } from '../organizations/organization.entity';
import { Candidate } from '../candidates/candidate.entity';
import { Resume } from '../resumes/resume.entity';
import { User } from '../users/user.entity';

@Entity('interview_sessions')
@Index(['organizationId', 'status'])
@Index(['organizationId', 'scheduledAt'])
export class InterviewSession {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  organizationId: string;

  @ManyToOne(() => Organization, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'organizationId' })
  organization: Organization;

  @Column({ nullable: true })
  candidateId: string | null;

  @ManyToOne(() => Candidate, { onDelete: 'SET NULL', nullable: true })
  @JoinColumn({ name: 'candidateId' })
  candidate: Candidate | null;

  @Column({ nullable: true })
  resumeId: string | null;

  @ManyToOne(() => Resume, { onDelete: 'SET NULL', nullable: true })
  @JoinColumn({ name: 'resumeId' })
  resume: Resume | null;

  @Column()
  createdById: string;

  @ManyToOne(() => User, { onDelete: 'RESTRICT' })
  @JoinColumn({ name: 'createdById' })
  createdBy: User;

  @Column()
  title: string;

  @Column({ nullable: true })
  jobTitle: string | null;

  @Column({ nullable: true })
  company: string | null;

  @Column({ type: 'enum', enum: InterviewType, default: InterviewType.GENERAL })
  interviewType: InterviewType;

  @Column({
    type: 'enum',
    enum: InterviewSessionStatus,
    default: InterviewSessionStatus.DRAFT,
  })
  status: InterviewSessionStatus;

  @Column({
    type: 'enum',
    enum: DesktopSyncStatus,
    default: DesktopSyncStatus.IDLE,
  })
  desktopSyncStatus: DesktopSyncStatus;

  @Column({ type: 'timestamptz', nullable: true })
  scheduledAt: Date | null;

  @Column({ type: 'timestamptz', nullable: true })
  startedAt: Date | null;

  @Column({ type: 'timestamptz', nullable: true })
  endedAt: Date | null;

  @Column({ default: false })
  consentCaptured: boolean;

  @Column({ type: 'text', nullable: true })
  notes: string | null;

  @Column({ type: 'int', default: 0 })
  lastTranscriptSequence: number;

  @CreateDateColumn()
  createdAt: Date;

  @UpdateDateColumn()
  updatedAt: Date;
}
