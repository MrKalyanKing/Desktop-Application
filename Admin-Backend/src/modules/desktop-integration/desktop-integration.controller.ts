import { Body, Controller, Get, Param, Post, Query } from '@nestjs/common';
import { ApiBearerAuth, ApiTags } from '@nestjs/swagger';
import { Type } from 'class-transformer';
import { IsArray, IsOptional, ValidateNested } from 'class-validator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { DesktopIntegrationService } from './desktop-integration.service';

class SegmentItemDto {
  eventId: string;
  speakerType: any;
  text: string;
}

class EventItemDto {
  eventId: string;
  eventType: any;
}

class SyncPushDto {
  @IsOptional()
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => SegmentItemDto)
  segments?: SegmentItemDto[];

  @IsOptional()
  @IsArray()
  @ValidateNested({ each: true })
  @Type(() => EventItemDto)
  events?: EventItemDto[];
}

@ApiTags('desktop')
@ApiBearerAuth()
@Controller('desktop')
export class DesktopIntegrationController {
  constructor(private readonly desktop: DesktopIntegrationService) {}

  @Post('heartbeat')
  heartbeat(@CurrentUser() user: AuthUser) {
    return this.desktop.heartbeat(user);
  }

  @Get('sessions')
  sessions(@CurrentUser() user: AuthUser) {
    return this.desktop.assignedSessions(user);
  }

  @Get('sync')
  status(@CurrentUser() user: AuthUser, @Query('sessionId') sessionId?: string) {
    return this.desktop.status(user, sessionId);
  }

  @Post('sync/:sessionId')
  push(
    @CurrentUser() user: AuthUser,
    @Param('sessionId') sessionId: string,
    @Body() dto: SyncPushDto,
  ) {
    return this.desktop.push(user, sessionId, dto as any);
  }
}
