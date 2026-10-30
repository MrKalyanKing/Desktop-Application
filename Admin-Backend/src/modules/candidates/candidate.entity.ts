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
import { Organization } from '../organizations/organization.entity';

@Entity('candidates')
@Index(['organizationId', 'email'])
export class Candidate {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  organizationId: string;

  @ManyToOne(() => Organization, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'organizationId' })
  organization: Organization;

  @Column()
  firstName: string;

  @Column()
  lastName: string;

  @Column({ nullable: true })
  email: string | null;

  @Column({ nullable: true })
  phone: string | null;

  @Column({ nullable: true })
  currentTitle: string | null;

  @Column({ nullable: true })
  currentCompany: string | null;

  @Column({ type: 'jsonb', default: [] })
  skills: string[];

  @Column({ type: 'text', nullable: true })
  notes: string | null;

  @Column({ default: false })
  consentGiven: boolean;

  @Column({ type: 'timestamptz', nullable: true })
  consentAt: Date | null;

  @Column({ type: 'date', nullable: true })
  retentionUntil: string | null;

  @CreateDateColumn()
  createdAt: Date;

  @UpdateDateColumn()
  updatedAt: Date;
}
