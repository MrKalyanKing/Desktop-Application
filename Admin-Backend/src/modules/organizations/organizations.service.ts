import { Injectable, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { Organization } from './organization.entity';

@Injectable()
export class OrganizationsService {
  constructor(
    @InjectRepository(Organization) private readonly orgs: Repository<Organization>,
  ) {}

  async current(organizationId: string) {
    const org = await this.orgs.findOne({ where: { id: organizationId } });
    if (!org) throw new NotFoundException('Organization not found');
    return org;
  }

  async update(
    organizationId: string,
    patch: Partial<Pick<Organization, 'name' | 'consentRequired' | 'retentionDays' | 'settings'>>,
  ) {
    const org = await this.current(organizationId);
    Object.assign(org, patch);
    return this.orgs.save(org);
  }
}
