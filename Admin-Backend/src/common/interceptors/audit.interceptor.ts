import {
  CallHandler,
  ExecutionContext,
  Injectable,
  NestInterceptor,
} from '@nestjs/common';
import { Observable, tap } from 'rxjs';
import { AuditLogsService } from '../../modules/audit-logs/audit-logs.service';

const WRITE_METHODS = new Set(['POST', 'PATCH', 'PUT', 'DELETE']);

@Injectable()
export class AuditInterceptor implements NestInterceptor {
  constructor(private readonly auditLogs: AuditLogsService) {}

  intercept(context: ExecutionContext, next: CallHandler): Observable<unknown> {
    const request = context.switchToHttp().getRequest();
    const method = request.method as string;
    if (!WRITE_METHODS.has(method) || request.path?.includes('/auth/login')) {
      return next.handle();
    }

    return next.handle().pipe(
      tap({
        next: () => {
          const user = request.user;
          if (!user) return;
          void this.auditLogs.record({
            organizationId: user.organizationId,
            userId: user.id,
            action: `${method} ${request.path}`,
            resourceType: request.path?.split('/')[3] ?? 'unknown',
            resourceId: request.params?.id,
            ip: request.ip,
            userAgent: request.headers['user-agent'],
            metadata: { query: request.query },
          });
        },
      }),
    );
  }
}
