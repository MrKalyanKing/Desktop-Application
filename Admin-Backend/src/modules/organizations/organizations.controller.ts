import { Body, Controller, Get, Patch } from '@nestjs/common';
import { ApiBearerAuth, ApiTags } from '@nestjs/swagger';
import { IsBoolean, IsInt, IsOptional, IsString } from 'class-validator';
import { CurrentUser } from '../../common/decorators/current-user.decorator';
import { Roles } from '../../common/decorators/roles.decorator';
import { UserRole } from '../../common/enums';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { OrganizationsService } from './organizations.service';

class UpdateOrgDto {
  @IsOptional()
  @IsString()
  name?: string;
  @IsOptional()
  @IsBoolean()
  consentRequired?: boolean;
  @IsOptional()
  @IsInt()
  retentionDays?: number;
}

@ApiTags('organizations')
@ApiBearerAuth()
@Controller('organizations')
export class OrganizationsController {
  constructor(private readonly orgs: OrganizationsService) {}

  @Get('current')
  current(@CurrentUser() user: AuthUser) {
    return this.orgs.current(user.organizationId);
  }

  @Roles(UserRole.OWNER, UserRole.ADMIN)
  @Patch('current')
  update(@CurrentUser() user: AuthUser, @Body() dto: UpdateOrgDto) {
    return this.orgs.update(user.organizationId, dto);
  }
}
