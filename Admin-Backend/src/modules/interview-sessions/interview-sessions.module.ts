import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { InterviewSession } from './interview-session.entity';
import { InterviewSessionsController } from './interview-sessions.controller';
import { InterviewSessionsService } from './interview-sessions.service';

@Module({
  imports: [TypeOrmModule.forFeature([InterviewSession])],
  controllers: [InterviewSessionsController],
  providers: [InterviewSessionsService],
  exports: [InterviewSessionsService],
})
export class InterviewSessionsModule {}
