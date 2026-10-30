import { Injectable, UnauthorizedException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { DeviceStatus, DesktopSyncStatus, InterviewSessionStatus } from '../../common/enums';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { InterviewSession } from '../interview-sessions/interview-session.entity';
import { TranscriptionService, SessionEventInput, TranscriptSegmentInput } from '../transcription/transcription.service';
import { DesktopDevice } from './desktop-device.entity';
import { SyncRecord } from './sync-record.entity';

@Injectable()
export class DesktopIntegrationService {
  constructor(
    @InjectRepository(DesktopDevice) private readonly devices: Repository<DesktopDevice>,
    @InjectRepository(SyncRecord) private readonly syncRecords: Repository<SyncRecord>,
    @InjectRepository(InterviewSession) private readonly sessions: Repository<InterviewSession>,
    private readonly transcription: TranscriptionService,
  ) {}

  async heartbeat(user: AuthUser) {
    if (!user.deviceId) throw new UnauthorizedException('Desktop device token required');
    const device = await this.devices.findOne({
      where: { id: user.deviceId, organizationId: user.organizationId, status: DeviceStatus.ACTIVE },
    });
    if (!device) throw new UnauthorizedException('Device revoked');
    device.lastSeenAt = new Date();
    await this.devices.save(device);
    return { ok: true, deviceId: device.id, lastSeenAt: device.lastSeenAt };
  }

  async assignedSessions(user: AuthUser) {
    return this.sessions.find({
      where: [
        { organizationId: user.organizationId, createdById: user.id },
        { organizationId: user.organizationId, status: InterviewSessionStatus.IN_PROGRESS },
        { organizationId: user.organizationId, status: InterviewSessionStatus.SCHEDULED },
      ],
      relations: ['candidate', 'resume'],
      order: { updatedAt: 'DESC' },
      take: 50,
    });
  }

  async status(user: AuthUser, sessionId?: string) {
    const device = user.deviceId
      ? await this.devices.findOne({ where: { id: user.deviceId } })
      : null;
    const records = await this.syncRecords.find({
      where: device
        ? sessionId
          ? { deviceId: device.id, sessionId }
          : { deviceId: device.id }
        : {},
      order: { updatedAt: 'DESC' },
      take: 20,
    });
    return {
      connected: Boolean(device && device.status === DeviceStatus.ACTIVE),
      device: device
        ? { id: device.id, name: device.deviceName, lastSeenAt: device.lastSeenAt, lastSyncAt: device.lastSyncAt }
        : null,
      records,
    };
  }

  async push(
    user: AuthUser,
    sessionId: string,
    payload: { segments?: TranscriptSegmentInput[]; events?: SessionEventInput[] },
  ) {
    if (!user.deviceId) throw new UnauthorizedException('Desktop device token required');
    const session = await this.sessions.findOne({
      where: { id: sessionId, organizationId: user.organizationId },
    });
    if (!session) throw new UnauthorizedException('Session is not available to this device');

    const segmentResult = payload.segments?.length
      ? await this.transcription.ingestSegments(user.organizationId, sessionId, payload.segments)
      : { accepted: 0, duplicates: 0, lastSequence: session.lastTranscriptSequence };
    const eventResult = payload.events?.length
      ? await this.transcription.ingestEvents(user.organizationId, sessionId, payload.events)
      : { accepted: 0, duplicates: 0 };

    session.desktopSyncStatus = DesktopSyncStatus.SYNCED;
    await this.sessions.save(session);

    let record = await this.syncRecords.findOne({
      where: { deviceId: user.deviceId, sessionId },
    });
    if (!record) {
      record = this.syncRecords.create({ deviceId: user.deviceId, sessionId });
    }
    record.lastSequence = segmentResult.lastSequence ?? record.lastSequence;
    record.pendingCount = 0;
    record.lastError = null;
    record.lastSyncedAt = new Date();
    await this.syncRecords.save(record);

    const device = await this.devices.findOneByOrFail({ id: user.deviceId });
    device.lastSyncAt = new Date();
    device.lastSeenAt = new Date();
    await this.devices.save(device);

    return {
      sessionId,
      syncStatus: session.desktopSyncStatus,
      transcript: segmentResult,
      events: eventResult,
    };
  }
}
