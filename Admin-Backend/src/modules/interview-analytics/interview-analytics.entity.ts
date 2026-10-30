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
import { AnalyticsKind, AnalyticsOrigin, HumanReviewStatus } from '../../common/enums';
import { InterviewSession } from '../interview-sessions/interview-session.entity';

@Entity('interview_analytics')
@Index(['sessionId', 'kind'])
export class InterviewAnalytics {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  sessionId: string;

  @ManyToOne(() => InterviewSession, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'sessionId' })
  session: InterviewSession;

  @Column({ type: 'enum', enum: AnalyticsKind })
  kind: AnalyticsKind;

  @Column({ type: 'enum', enum: AnalyticsOrigin, default: AnalyticsOrigin.AI_GENERATED })
  origin: AnalyticsOrigin;

  @Column({ default: 'heuristic' })
  provider: string;

  @Column({ nullable: true })
  model: string | null;

  @Column({ type: 'jsonb' })
  payload: Record<string, unknown>;

  @Column({ default: true })
  isAiGenerated: boolean;

  @Column({
    type: 'enum',
    enum: HumanReviewStatus,
    default: HumanReviewStatus.PENDING,
  })
  humanReviewStatus: HumanReviewStatus;

  @Column({ type: 'text', nullable: true })
  humanReviewNotes: string | null;

  @Column({ nullable: true })
  reviewedById: string | null;

  @CreateDateColumn()
  generatedAt: Date;

  @UpdateDateColumn()
  updatedAt: Date;
}
