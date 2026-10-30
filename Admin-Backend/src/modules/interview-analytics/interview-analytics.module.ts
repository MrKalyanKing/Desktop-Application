import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { InterviewSessionsModule } from '../interview-sessions/interview-sessions.module';
import { TranscriptSegment } from '../transcription/transcript-segment.entity';
import { InterviewAnalytics } from './interview-analytics.entity';
import { InterviewAnalyticsController } from './interview-analytics.controller';
import { InterviewAnalyticsService } from './interview-analytics.service';

@Module({
  imports: [
    TypeOrmModule.forFeature([InterviewAnalytics, TranscriptSegment]),
    InterviewSessionsModule,
  ],
  controllers: [InterviewAnalyticsController],
  providers: [InterviewAnalyticsService],
  exports: [InterviewAnalyticsService],
})
export class InterviewAnalyticsModule {}
