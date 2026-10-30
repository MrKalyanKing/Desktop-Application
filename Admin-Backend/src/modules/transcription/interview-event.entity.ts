import {
  Column,
  CreateDateColumn,
  Entity,
  Index,
  JoinColumn,
  ManyToOne,
  PrimaryGeneratedColumn,
} from 'typeorm';
import { InterviewEventType } from '../../common/enums';
import { InterviewSession } from '../interview-sessions/interview-session.entity';

@Entity('interview_events')
@Index(['sessionId', 'eventId'], { unique: true })
@Index(['sessionId', 'occurredAt'])
export class InterviewEvent {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  sessionId: string;

  @ManyToOne(() => InterviewSession, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'sessionId' })
  session: InterviewSession;

  @Column({ type: 'enum', enum: InterviewEventType })
  eventType: InterviewEventType;

  @Column()
  eventId: string;

  @Column({ type: 'jsonb', default: {} })
  payload: Record<string, unknown>;

  @Column({ type: 'timestamptz' })
  occurredAt: Date;

  @CreateDateColumn()
  receivedAt: Date;
}
