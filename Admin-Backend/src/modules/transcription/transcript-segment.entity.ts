import {
  Column,
  CreateDateColumn,
  Entity,
  Index,
  JoinColumn,
  ManyToOne,
  PrimaryGeneratedColumn,
} from 'typeorm';
import { SpeakerType, TranscriptSource } from '../../common/enums';
import { InterviewSession } from '../interview-sessions/interview-session.entity';

@Entity('transcript_segments')
@Index(['sessionId', 'sequence'])
@Index(['sessionId', 'eventId'], { unique: true })
export class TranscriptSegment {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  sessionId: string;

  @ManyToOne(() => InterviewSession, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'sessionId' })
  session: InterviewSession;

  @Column({ type: 'enum', enum: SpeakerType, default: SpeakerType.UNKNOWN })
  speakerType: SpeakerType;

  @Column({ type: 'text' })
  text: string;

  @Column({ type: 'int', default: 0 })
  startMs: number;

  @Column({ type: 'int', default: 0 })
  endMs: number;

  @Column({ type: 'int' })
  sequence: number;

  @Column({ type: 'enum', enum: TranscriptSource, default: TranscriptSource.DESKTOP_GEMINI })
  source: TranscriptSource;

  @Column({ type: 'float', nullable: true })
  confidence: number | null;

  @Column()
  eventId: string;

  @Column({ type: 'jsonb', nullable: true })
  metadata: Record<string, unknown> | null;

  @CreateDateColumn()
  createdAt: Date;
}
