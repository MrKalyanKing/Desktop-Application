import {
  Column,
  CreateDateColumn,
  Entity,
  Index,
  JoinColumn,
  ManyToOne,
  PrimaryGeneratedColumn,
} from 'typeorm';
import { SpeakerType } from '../../common/enums';
import { InterviewSession } from '../interview-sessions/interview-session.entity';
import { InterviewQuestion } from '../interview-questions/interview-question.entity';

@Entity('interview_responses')
@Index(['sessionId', 'questionId'])
@Index(['sessionId', 'eventId'], { unique: true, where: '"eventId" IS NOT NULL' })
export class InterviewResponse {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  sessionId: string;

  @ManyToOne(() => InterviewSession, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'sessionId' })
  session: InterviewSession;

  @Column({ nullable: true })
  questionId: string | null;

  @ManyToOne(() => InterviewQuestion, { onDelete: 'SET NULL', nullable: true })
  @JoinColumn({ name: 'questionId' })
  question: InterviewQuestion | null;

  @Column({ type: 'enum', enum: SpeakerType, default: SpeakerType.CANDIDATE })
  speaker: SpeakerType;

  @Column({ type: 'text' })
  text: string;

  @Column({ type: 'int', nullable: true })
  startMs: number | null;

  @Column({ type: 'int', nullable: true })
  endMs: number | null;

  @Column({ type: 'int', nullable: true })
  durationMs: number | null;

  @Column({ nullable: true })
  eventId: string | null;

  @CreateDateColumn()
  createdAt: Date;
}
