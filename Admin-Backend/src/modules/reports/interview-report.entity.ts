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
import { ReportStatus } from '../../common/enums';
import { InterviewSession } from '../interview-sessions/interview-session.entity';
import { User } from '../users/user.entity';

@Entity('interview_reports')
@Index(['sessionId', 'templateKey'])
export class InterviewReport {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  sessionId: string;

  @ManyToOne(() => InterviewSession, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'sessionId' })
  session: InterviewSession;

  @Column({ default: 'session_summary' })
  templateKey: string;

  @Column({ type: 'enum', enum: ReportStatus, default: ReportStatus.PENDING })
  status: ReportStatus;

  @Column({ nullable: true })
  storageKey: string | null;

  @Column({ nullable: true })
  mimeType: string | null;

  @Column({ type: 'text', nullable: true })
  error: string | null;

  @Column({ nullable: true })
  createdById: string | null;

  @ManyToOne(() => User, { onDelete: 'SET NULL', nullable: true })
  @JoinColumn({ name: 'createdById' })
  createdBy: User | null;

  @CreateDateColumn()
  createdAt: Date;

  @UpdateDateColumn()
  generatedAt: Date;
}
