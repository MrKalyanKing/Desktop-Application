import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { InterviewQuestion } from '../interview-questions/interview-question.entity';
import { InterviewResponse } from '../interview-responses/interview-response.entity';
import { InterviewSessionsModule } from '../interview-sessions/interview-sessions.module';
import { InterviewEvent } from './interview-event.entity';
import { TranscriptSegment } from './transcript-segment.entity';
import { TranscriptionController } from './transcription.controller';
import { TranscriptionService } from './transcription.service';

@Module({
  imports: [
    TypeOrmModule.forFeature([
      TranscriptSegment,
      InterviewEvent,
      InterviewQuestion,
      InterviewResponse,
    ]),
    InterviewSessionsModule,
  ],
  controllers: [TranscriptionController],
  providers: [TranscriptionService],
  exports: [TranscriptionService],
})
export class TranscriptionModule {}
