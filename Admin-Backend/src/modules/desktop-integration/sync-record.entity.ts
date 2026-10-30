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
import { DesktopDevice } from './desktop-device.entity';
import { InterviewSession } from '../interview-sessions/interview-session.entity';

@Entity('sync_records')
@Index(['deviceId', 'sessionId'], { unique: true })
export class SyncRecord {
  @PrimaryGeneratedColumn('uuid')
  id: string;

  @Column()
  deviceId: string;

  @ManyToOne(() => DesktopDevice, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'deviceId' })
  device: DesktopDevice;

  @Column()
  sessionId: string;

  @ManyToOne(() => InterviewSession, { onDelete: 'CASCADE' })
  @JoinColumn({ name: 'sessionId' })
  session: InterviewSession;

  @Column({ type: 'int', default: 0 })
  lastSequence: number;

  @Column({ type: 'int', default: 0 })
  pendingCount: number;

  @Column({ type: 'text', nullable: true })
  lastError: string | null;

  @Column({ type: 'timestamptz', nullable: true })
  lastSyncedAt: Date | null;

  @CreateDateColumn()
  createdAt: Date;

  @UpdateDateColumn()
  updatedAt: Date;
}
