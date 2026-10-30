import { Injectable } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import { AuditLog } from './audit-log.entity';

@Injectable()
export class AuditLogsService {
  constructor(@InjectRepository(AuditLog) private readonly logs: Repository<AuditLog>) {}

  record(entry: Partial<AuditLog>) {
    return this.logs.save(this.logs.create(entry));
  }

  list(organizationId: string) {
    return this.logs.find({
      where: { organizationId },
      order: { createdAt: 'DESC' },
      take: 200,
    });
  }
}
