import {
  Column,
  CreateDateColumn,
  Entity,
  Index,
  JoinColumn,
  ManyToOne,
  OneToMany,
  PrimaryGeneratedColumn,
  UpdateDateColumn,
} from 'typeorm';
import { Organization } from '../organizations/organization.entity';
import { Candidate } from '../candidates/candidate.entity';
import { ResumeVersion } from './resume-version.entity';

@Entity('resumes')
@Index(['organizationId', 'candidateId'])
export class Resume {
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

  @Column()
  originalFilename: string;

  @Column()
  mimeType: string;

  @Column({ type: 'int' })
  sizeBytes: number;

  @Column()
  storageKey: string;

  @Column({ default: 'local' })
  storageProvider: string;

  @Column({ type: 'text', nullable: true })
  extractedText: string | null;

  @Column({ type: 'jsonb', nullable: true })
  extractedJson: Record<string, unknown> | null;

  @Column({ default: 'uploaded' })
  status: string;

  @Column({ default: 1 })
  currentVersion: number;

  @OneToMany(() => ResumeVersion, (version) => version.resume)
  versions: ResumeVersion[];

  @CreateDateColumn()
  createdAt: Date;

  @UpdateDateColumn()
  updatedAt: Date;
}
