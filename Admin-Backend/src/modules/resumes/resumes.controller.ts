import {
  Body,
  Controller,
  Delete,
  Get,
  Param,
  Patch,
  Post,
  Query,
  Res,
  UploadedFile,
  UseInterceptors,
} from '@nestjs/common';
import { FileInterceptor } from '@nestjs/platform-express';
import { ApiBearerAuth, ApiBody, ApiConsumes, ApiTags } from '@nestjs/swagger';
import type { Response } from 'express';
import { memoryStorage } from 'multer';
import { IsOptional, IsUUID } from 'class-validator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { ResumesService } from './resumes.service';

class AssociateDto {
  @IsUUID()
  candidateId: string;
}

class OptionalCandidateDto {
  @IsOptional()
  @IsUUID()
  candidateId?: string;
}

@ApiTags('resumes')
@ApiBearerAuth()
@Controller('resumes')
export class ResumesController {
  constructor(private readonly resumes: ResumesService) {}

  @Get()
  list(@CurrentUser() user: AuthUser, @Query('search') search?: string) {
    return this.resumes.list(user.organizationId, search);
  }

  @Get(':id')
  get(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.resumes.get(user.organizationId, id);
  }

  @Get(':id/download')
  async download(
    @CurrentUser() user: AuthUser,
    @Param('id') id: string,
    @Res() res: Response,
  ) {
    const { buffer, resume } = await this.resumes.download(user.organizationId, id);
    res.setHeader('Content-Type', resume.mimeType);
    res.setHeader('Content-Disposition', `attachment; filename="${resume.originalFilename}"`);
    res.send(buffer);
  }

  @Post()
  @ApiConsumes('multipart/form-data')
  @ApiBody({ schema: { type: 'object', properties: { file: { type: 'string', format: 'binary' } } } })
  @UseInterceptors(FileInterceptor('file', { storage: memoryStorage() }))
  upload(
    @CurrentUser() user: AuthUser,
    @UploadedFile() file: Express.Multer.File,
    @Body() body: OptionalCandidateDto,
  ) {
    return this.resumes.upload(user.organizationId, user.id, file, body.candidateId);
  }

  @Post(':id/versions')
  @UseInterceptors(FileInterceptor('file', { storage: memoryStorage() }))
  addVersion(
    @CurrentUser() user: AuthUser,
    @Param('id') id: string,
    @UploadedFile() file: Express.Multer.File,
  ) {
    return this.resumes.addVersion(user.organizationId, id, user.id, file);
  }

  @Patch(':id/candidate')
  associate(@CurrentUser() user: AuthUser, @Param('id') id: string, @Body() dto: AssociateDto) {
    return this.resumes.associateCandidate(user.organizationId, id, dto.candidateId);
  }

  @Delete(':id')
  remove(@CurrentUser() user: AuthUser, @Param('id') id: string) {
    return this.resumes.remove(user.organizationId, id);
  }
}
