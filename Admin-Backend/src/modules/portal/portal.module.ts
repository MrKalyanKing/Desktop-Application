import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { AuthModule } from '../auth/auth.module';
import { Candidate } from '../candidates/candidate.entity';
import { InterviewAnalyticsModule } from '../interview-analytics/interview-analytics.module';
import { InterviewSessionsModule } from '../interview-sessions/interview-sessions.module';
import { ResumesModule } from '../resumes/resumes.module';
import { TranscriptionModule } from '../transcription/transcription.module';
import { PortalController } from './portal.controller';
import { PortalService } from './portal.service';

@Module({
  imports: [
    TypeOrmModule.forFeature([Candidate]),
    AuthModule,
    ResumesModule,
    InterviewSessionsModule,
    TranscriptionModule,
    InterviewAnalyticsModule,
  ],
  controllers: [PortalController],
  providers: [PortalService],
})
export class PortalModule {}
