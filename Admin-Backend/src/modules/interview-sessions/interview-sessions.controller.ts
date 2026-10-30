import { Body, Controller, Delete, Get, Param, Patch, Post, Query } from '@nestjs/common';
import { ApiBearerAuth, ApiTags } from '@nestjs/swagger';
import {
  IsBoolean,
  IsEnum,
  IsOptional,
  IsString,
  IsUUID,
} from 'class-validator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import { InterviewSessionStatus, InterviewType } from '../../common/enums';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { InterviewSessionsService } from './interview-sessions.service';

class CreateSessionDto {
  @IsString()
  title: string;
  @IsOptional()
  @IsString()
  jobTitle?: string;
  @IsOptional()
  @IsString()
  company?: string;
  @IsOptional()
  @IsEnum(InterviewType)
  interviewType?: InterviewType;
  @IsOptional()
  @IsUUID()
  candidateId?: string;
  @IsOptional()
  @IsUUID()
  resumeId?: string;
  @IsOptional()
  @IsString()
  scheduledAt?: string;
  @IsOptional()
  @IsString()
  notes?: string;
  @IsOptional()
  @IsBoolean()
  consentCaptured?: boolean;
}

@ApiTags('interview-sessions')
@ApiBearerAuth()
@Controller('interview-sessions')
export class InterviewSessionsController {
  constructor(private readonly sessions: InterviewSessionsService) {}

  @Get()
  list(
    @CurrentUser() user: AuthUser,
    @Query('status') status?: InterviewSessionStatus,
    @Query('search') search?: string,
    @Query('type') type?: InterviewType,
  ) {
    return this.sessions.list(user.organizationId, { status, search, type });
  }

  @Get(':id')
  get(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.sessions.get(user.organizationId, id);
  }

  @Post()
  create(@CurrentUser() user: AuthUser, @Body() dto: CreateSessionDto) {
    return this.sessions.create(user.organizationId, user.id, dto);
  }

  @Patch(':id')
  update(@CurrentUser() user: AuthUser, @Param('id') id: string, @Body() dto: CreateSessionDto) {
    return this.sessions.update(user.organizationId, id, dto);
  }

  @Post(':id/start')
  start(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.sessions.start(user.organizationId, id);
  }

  @Post(':id/complete')
  complete(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.sessions.complete(user.organizationId, id);
  }

  @Post(':id/archive')
  archive(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.sessions.archive(user.organizationId, id);
  }

  @Delete(':id')
  remove(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.sessions.remove(user.organizationId, id);
  }
}
