import {
  Body,
  Controller,
  Get,
  Param,
  Patch,
  Post,
  UploadedFile,
  UseInterceptors,
} from '@nestjs/common';
import { FileInterceptor } from '@nestjs/platform-express';
import { ApiBearerAuth, ApiTags } from '@nestjs/swagger';
import { IsArray, IsBoolean, IsEnum, IsOptional, IsString, IsUUID } from 'class-validator';
import { memoryStorage } from 'multer';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import { Public } from '../../common/decorators/public.decorator';
import { Roles } from '../../common/decorators/roles.decorator';
import { InterviewType, UserRole } from '../../common/enums';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { LoginDto, RegisterCustomerDto } from '../auth/dto/auth.dto';
import { PortalService } from './portal.service';

class PracticeDto {
  @IsOptional()
  @IsEnum(InterviewType)
  interviewType?: InterviewType;
  @IsOptional()
  @IsString()
  jobTitle?: string;
  @IsOptional()
  @IsString()
  company?: string;
  @IsOptional()
  @IsUUID()
  resumeId?: string;
}

class AnswerDto {
  @IsUUID()
  questionId: string;
  @IsString()
  text: string;
}

class ProfileDto {
  @IsOptional()
  @IsString()
  currentTitle?: string;
  @IsOptional()
  @IsString()
  currentCompany?: string;
  @IsOptional()
  @IsString()
  phone?: string;
  @IsOptional()
  @IsArray()
  skills?: string[];
  @IsOptional()
  @IsBoolean()
  consentGiven?: boolean;
}

@ApiTags('portal')
@Controller('portal')
export class PortalController {
  constructor(private readonly portal: PortalService) {}

  @Public()
  @Post('auth/register')
  register(@Body() dto: RegisterCustomerDto) {
    return this.portal.register(dto);
  }

  @Public()
  @Post('auth/login')
  login(@Body() dto: LoginDto) {
    return this.portal.login(dto);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Get('me')
  me(@CurrentUser() user: AuthUser) {
    return this.portal.me(user);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Patch('me')
  update(@CurrentUser() user: AuthUser, @Body() dto: ProfileDto) {
    return this.portal.updateProfile(user, dto);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Get('resumes')
  resumes(@CurrentUser() user: AuthUser) {
    return this.portal.resumesFor(user);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Post('resumes')
  @UseInterceptors(FileInterceptor('file', { storage: memoryStorage() }))
  upload(@CurrentUser() user: AuthUser, @UploadedFile() file: Express.Multer.File) {
    return this.portal.uploadResume(user, file);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Get('interviews')
  interviews(@CurrentUser() user: AuthUser) {
    return this.portal.interviews(user);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Get('interviews/:id')
  interview(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.portal.interview(user, id);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Post('interviews/practice')
  practice(@CurrentUser() user: AuthUser, @Body() dto: PracticeDto) {
    return this.portal.startPractice(user, dto);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Post('interviews/:id/answers')
  answer(
    @CurrentUser() user: AuthUser,
    @Param('id') id: string,
    @Body() dto: AnswerDto,
  ) {
    return this.portal.answer(user, id, dto.questionId, dto.text);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Post('interviews/:id/complete')
  complete(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.portal.complete(user, id);
  }

  @ApiBearerAuth()
  @Roles(UserRole.CANDIDATE)
  @Get('performance')
  performance(@CurrentUser() user: AuthUser) {
    return this.portal.performance(user);
  }
}
