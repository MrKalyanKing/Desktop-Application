import { Body, Controller, Delete, Get, Param, Patch, Post, Query } from '@nestjs/common';
import { ApiBearerAuth, ApiTags } from '@nestjs/swagger';
import { IsArray, IsBoolean, IsEmail, IsOptional, IsString } from 'class-validator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { CandidatesService } from './candidates.service';

class CandidateDto {
  @IsString()
  firstName: string;
  @IsString()
  lastName: string;
  @IsOptional()
  @IsEmail()
  email?: string;
  @IsOptional()
  @IsString()
  phone?: string;
  @IsOptional()
  @IsString()
  currentTitle?: string;
  @IsOptional()
  @IsString()
  currentCompany?: string;
  @IsOptional()
  @IsArray()
  skills?: string[];
  @IsOptional()
  @IsString()
  notes?: string;
  @IsOptional()
  @IsBoolean()
  consentGiven?: boolean;
}

@ApiTags('candidates')
@ApiBearerAuth()
@Controller('candidates')
export class CandidatesController {
  constructor(private readonly candidates: CandidatesService) {}

  @Get()
  list(@CurrentUser() user: AuthUser, @Query('search') search?: string) {
    return this.candidates.list(user.organizationId, search);
  }

  @Get(':id')
  get(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.candidates.get(user.organizationId, id);
  }

  @Post()
  create(@CurrentUser() user: AuthUser, @Body() dto: CandidateDto) {
    return this.candidates.create(user.organizationId, {
      ...dto,
      consentAt: dto.consentGiven ? new Date() : null,
    });
  }

  @Patch(':id')
  update(@CurrentUser() user: AuthUser, @Param('id') id: string, @Body() dto: CandidateDto) {
    return this.candidates.update(user.organizationId, id, dto);
  }

  @Delete(':id')
  remove(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.candidates.remove(user.organizationId, id);
  }
}
