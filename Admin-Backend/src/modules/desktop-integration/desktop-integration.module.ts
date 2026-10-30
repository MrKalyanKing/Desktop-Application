import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { InterviewSession } from '../interview-sessions/interview-session.entity';
import { TranscriptionModule } from '../transcription/transcription.module';
import { DesktopDevice } from './desktop-device.entity';
import { DesktopIntegrationController } from './desktop-integration.controller';
import { DesktopIntegrationService } from './desktop-integration.service';
import { SyncRecord } from './sync-record.entity';

@Module({
  imports: [
    TypeOrmModule.forFeature([DesktopDevice, SyncRecord, InterviewSession]),
    TranscriptionModule,
  ],
  controllers: [DesktopIntegrationController],
  providers: [DesktopIntegrationService],
  exports: [DesktopIntegrationService],
})
export class DesktopIntegrationModule {}
