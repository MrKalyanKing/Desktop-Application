import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { InterviewQuestion } from './interview-question.entity';

@Module({
  imports: [TypeOrmModule.forFeature([InterviewQuestion])],
  exports: [TypeOrmModule],
})
export class InterviewQuestionsModule {}
