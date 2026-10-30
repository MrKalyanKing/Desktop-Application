import { Body, Controller, Get, Param, Post, Query, Res } from '@nestjs/common';
import { ApiBearerAuth, ApiTags } from '@nestjs/swagger';
import type { Response } from 'express';
import { IsOptional, IsString } from 'class-validator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { ReportsService } from './reports.service';

class GenerateReportDto {
  @IsOptional()
  @IsString()
  templateKey?: string;
}

@ApiTags('reports')
@ApiBearerAuth()
@Controller()
export class ReportsController {
  constructor(private readonly reports: ReportsService) {}

  @Get('reports')
  list(@CurrentUser() user: AuthUser, @Query('sessionId') sessionId?: string) {
    return this.reports.list(user.organizationId, sessionId);
  }

  @Post('interview-sessions/:id/reports')
  generate(
    @CurrentUser() user: AuthUser,
    @Param('id') id: string,
    @Body() dto: GenerateReportDto,
  ) {
    return this.reports.generate(user.organizationId, id, user.id, dto.templateKey);
  }

  @Get('reports/:id/download')
  async download(
    @CurrentUser() user: AuthUser,
    @Param('id') id: string,
    @Res() res: Response,
  ) {
    const result = await this.reports.download(user.organizationId, id);
    if (!result) {
      res.status(404).json({ message: 'Report not ready' });
      return;
    }
    res.setHeader('Content-Type', result.report.mimeType ?? 'application/pdf');
    res.setHeader('Content-Disposition', `attachment; filename="${result.report.id}.pdf"`);
    res.send(result.buffer);
  }
}
