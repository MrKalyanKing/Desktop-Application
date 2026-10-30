import {
  Column,
  CreateDateColumn,
  Entity,
  Index,
  JoinColumn,
  ManyToOne,
  PrimaryGeneratedColumn,
} from 'typeorm';
import { QuestionAskedBy } from '../../common/enums';
import { InterviewSession } from '../interview-sessions/interview-session.entity';

@Entity('interview_questions')
@Index(['sessionId', 'sequence'])
@Index(['sessionId', 'eventId'], { unique: true, where: '"eventId" IS NOT NULL' })
export class InterviewQuestion {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  sessionId: string;

  @ManyToOne(() => InterviewSession, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'sessionId' })
  session: InterviewSession;

  @Column({ type: 'enum', enum: QuestionAskedBy, default: QuestionAskedBy.INTERVIEWER })
  askedBy: QuestionAskedBy;

  @Column({ type: 'text' })
  text: string;

  @Column({ nullable: true })
  category: string | null;

  @Column({ type: 'int' })
  sequence: number;

  @Column({ type: 'timestamptz', nullable: true })
  askedAt: Date | null;

  @Column({ nullable: true })
  eventId: string | null;

  @CreateDateColumn()
  createdAt: Date;
}
