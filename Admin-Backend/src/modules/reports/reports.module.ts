import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { FileStorageModule } from '../file-storage/file-storage.module';
import { InterviewAnalytics } from '../interview-analytics/interview-analytics.entity';
import { InterviewSessionsModule } from '../interview-sessions/interview-sessions.module';
import { TranscriptSegment } from '../transcription/transcript-segment.entity';
import { InterviewReport } from './interview-report.entity';
import { ReportsController } from './reports.controller';
import { ReportsService } from './reports.service';

@Module({
  imports: [
    TypeOrmModule.forFeature([InterviewReport, TranscriptSegment, InterviewAnalytics]),
    InterviewSessionsModule,
    FileStorageModule,
  ],
  controllers: [ReportsController],
  providers: [ReportsService],
  exports: [ReportsService],
})
export class ReportsModule {}
