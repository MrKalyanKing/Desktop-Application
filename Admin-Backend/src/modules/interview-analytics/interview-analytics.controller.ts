import { Body, Controller, Get, Param, Patch, Post } from '@nestjs/common';
import { ApiBearerAuth, ApiTags } from '@nestjs/swagger';
import { IsEnum, IsOptional, IsString } from 'class-validator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import { HumanReviewStatus } from '../../common/enums';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { InterviewAnalyticsService } from './interview-analytics.service';

class ReviewDto {
  @IsEnum(HumanReviewStatus)
  status: HumanReviewStatus;
  @IsOptional()
  @IsString()
  notes?: string;
}

@ApiTags('analytics')
@ApiBearerAuth()
@Controller()
export class InterviewAnalyticsController {
  constructor(private readonly analytics: InterviewAnalyticsService) {}

  @Get('interview-sessions/:id/analytics')
  list(@Param('id') id: string) {
    return this.analytics.list(id);
  }

  @Post('interview-sessions/:id/analytics/generate')
  generate(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.analytics.generate(user.organizationId, id);
  }

  @Patch('analytics/:id/review')
  review(@CurrentUser() user: AuthUser, @Param('id') id: string, @Body() dto: ReviewDto) {
    return this.analytics.review(user.organizationId, id, dto.status, dto.notes);
  }
}
