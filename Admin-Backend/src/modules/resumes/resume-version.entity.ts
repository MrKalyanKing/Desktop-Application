import {
  Column,
  CreateDateColumn,
  Entity,
  Index,
  JoinColumn,
  ManyToOne,
  PrimaryGeneratedColumn,
} from 'typeorm';
import { Resume } from './resume.entity';
import { User } from '../users/user.entity';

@Entity('resume_versions')
@Index(['resumeId', 'versionNumber'], { unique: true })
export class ResumeVersion {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  resumeId: string;

  @ManyToOne(() => Resume, (resume) => resume.versions, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'resumeId' })
  resume: Resume;

  @Column({ type: 'int' })
  versionNumber: number;

  @Column()
  storageKey: string;

  @Column()
  originalFilename: string;

  @Column({ type: 'text', nullable: true })
  extractedText: string | null;

  @Column({ type: 'jsonb', nullable: true })
  extractedJson: Record<string, unknown> | null;

  @Column({ nullable: true })
  createdById: string | null;

  @ManyToOne(() => User, { onDelete: 'SET NULL', nullable: true })
  @JoinColumn({ name: 'createdById' })
  createdBy: User | null;

  @CreateDateColumn()
  createdAt: Date;
}
