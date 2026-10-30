import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { Candidate } from '../candidates/candidate.entity';
import { DesktopDevice } from '../desktop-integration/desktop-device.entity';
import { InterviewSession } from '../interview-sessions/interview-session.entity';
import { Resume } from '../resumes/resume.entity';
import { DashboardController } from './dashboard.controller';
import { DashboardService } from './dashboard.service';

@Module({
  imports: [TypeOrmModule.forFeature([InterviewSession, Resume, Candidate, DesktopDevice])],
  controllers: [DashboardController],
  providers: [DashboardService],
})
export class DashboardModule {}
