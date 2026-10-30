import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { InterviewResponse } from './interview-response.entity';

@Module({
  imports: [TypeOrmModule.forFeature([InterviewResponse])],
  exports: [TypeOrmModule],
})
export class InterviewResponsesModule {}
