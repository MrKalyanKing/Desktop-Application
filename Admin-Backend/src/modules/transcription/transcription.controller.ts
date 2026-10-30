import { Body, Controller, Get, Param, Post } from '@nestjs/common';
import { ApiBearerAuth, ApiTags } from '@nestjs/swagger';
import { Type } from 'class-transformer';
import {
  IsArray,
  IsEnum,
  IsInt,
  IsNumber,
  IsObject,
  IsOptional,
  IsString,
  IsUUID,
  ValidateNested,
} from 'class-validator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import {
  InterviewEventType,
  QuestionAskedBy,
  SpeakerType,
  TranscriptSource,
} from '../../common/enums';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { TranscriptionService } from './transcription.service';

class SegmentDto {
  @IsUUID()
  eventId: string;
  @IsEnum(SpeakerType)
  speakerType: SpeakerType;
  @IsString()
  text: string;
  @IsOptional()
  @IsInt()
  startMs?: number;
  @IsOptional()
  @IsInt()
  endMs?: number;
  @IsOptional()
  @IsInt()
  sequence?: number;
  @IsOptional()
  @IsEnum(TranscriptSource)
  source?: TranscriptSource;
  @IsOptional()
  @IsNumber()
  confidence?: number;
  @IsOptional()
  @IsObject()
  metadata?: Record<string, unknown>;
}

class BatchSegmentsDto {
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => SegmentDto)
  segments: SegmentDto[];
}

class EventDto {
  @IsUUID()
  eventId: string;
  @IsEnum(InterviewEventType)
  eventType: InterviewEventType;
  @IsOptional()
  @IsObject()
  payload?: Record<string, unknown>;
  @IsOptional()
  @IsString()
  occurredAt?: string;
}

class BatchEventsDto {
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => EventDto)
  events: EventDto[];
}

class QuestionDto {
  @IsString()
  text: string;
  @IsOptional()
  @IsEnum(QuestionAskedBy)
  askedBy?: QuestionAskedBy;
  @IsOptional()
  @IsString()
  category?: string;
}

@ApiTags('transcription')
@ApiBearerAuth()
@Controller('interview-sessions/:id')
export class TranscriptionController {
  constructor(private readonly transcription: TranscriptionService) {}

  @Get('transcript')
  async transcript(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    await this.transcription.assertSession(user.organizationId, id);
    return this.transcription.listSegments(id);
  }

  @Post('transcript')
  ingestTranscript(
    @CurrentUser() user: AuthUser,
    @Param('id') id: string,
    @Body() dto: BatchSegmentsDto,
  ) {
    return this.transcription.ingestSegments(user.organizationId, id, dto.segments);
  }

  @Get('events')
  events(@Param('id') id: string) {
    return this.transcription.listEvents(id);
  }

  @Post('events')
  ingestEvents(
    @CurrentUser() user: AuthUser,
    @Param('id') id: string,
    @Body() dto: BatchEventsDto,
  ) {
    return this.transcription.ingestEvents(user.organizationId, id, dto.events);
  }

  @Get('questions')
  questions(@Param('id') id: string) {
    return this.transcription.listQuestions(id);
  }

  @Post('questions')
  addQuestion(
    @CurrentUser() user: AuthUser,
    @Param('id') id: string,
    @Body() dto: QuestionDto,
  ) {
    return this.transcription.addQuestion(user.organizationId, id, dto);
  }

  @Get('responses')
  responses(@Param('id') id: string) {
    return this.transcription.listResponses(id);
  }
}
