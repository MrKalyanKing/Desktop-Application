import { Module } from '@nestjs/common';
import { ConfigModule, ConfigService } from '@nestjs/config';
import { APP_GUARD, APP_INTERCEPTOR } from '@nestjs/core';
import { ThrottlerGuard, ThrottlerModule } from '@nestjs/throttler';
import { TypeOrmModule } from '@nestjs/typeorm';
import configuration from './config/configuration';
import { JwtAuthGuard } from './common/guards/jwt-auth.guard';
import { RolesGuard } from './common/guards/roles.guard';
import { AuditInterceptor } from './common/interceptors/audit.interceptor';
import { ALL_ENTITIES } from './database/entities';
import { HealthController } from './health.controller';
import { AuthModule } from './modules/auth/auth.module';
import { UsersModule } from './modules/users/users.module';
import { OrganizationsModule } from './modules/organizations/organizations.module';
import { CandidatesModule } from './modules/candidates/candidates.module';
import { ResumesModule } from './modules/resumes/resumes.module';
import { InterviewSessionsModule } from './modules/interview-sessions/interview-sessions.module';
import { InterviewQuestionsModule } from './modules/interview-questions/interview-questions.module';
import { InterviewResponsesModule } from './modules/interview-responses/interview-responses.module';
import { TranscriptionModule } from './modules/transcription/transcription.module';
import { InterviewAnalyticsModule } from './modules/interview-analytics/interview-analytics.module';
import { ReportsModule } from './modules/reports/reports.module';
import { DesktopIntegrationModule } from './modules/desktop-integration/desktop-integration.module';
import { NotificationsModule } from './modules/notifications/notifications.module';
import { FileStorageModule } from './modules/file-storage/file-storage.module';
import { AuditLogsModule } from './modules/audit-logs/audit-logs.module';
import { DashboardModule } from './modules/dashboard/dashboard.module';
import { PortalModule } from './modules/portal/portal.module';

@Module({
  imports: [
    ConfigModule.forRoot({ isGlobal: true, load: [configuration] }),
    ThrottlerModule.forRootAsync({
      inject: [ConfigService],
      useFactory: (config: ConfigService) => [
        {
          ttl: config.get<number>('throttle.ttl') ?? 60000,
          limit: config.get<number>('throttle.limit') ?? 120,
        },
      ],
    }),
    TypeOrmModule.forRootAsync({
      inject: [ConfigService],
      useFactory: (config: ConfigService) => ({
        type: 'postgres',
        url: config.get<string>('database.url'),
        ssl: config.get('database.ssl') ? { rejectUnauthorized: false } : false,
        entities: ALL_ENTITIES,
        synchronize: config.get('database.synchronize'),
        logging: config.get('database.logging'),
      }),
    }),
    FileStorageModule,
    AuthModule,
    UsersModule,
    OrganizationsModule,
    CandidatesModule,
    ResumesModule,
    InterviewSessionsModule,
    InterviewQuestionsModule,
    InterviewResponsesModule,
    TranscriptionModule,
    InterviewAnalyticsModule,
    ReportsModule,
    DesktopIntegrationModule,
    NotificationsModule,
    AuditLogsModule,
    DashboardModule,
    PortalModule,
  ],
  controllers: [HealthController],
  providers: [
    { provide: APP_GUARD, useClass: JwtAuthGuard },
    { provide: APP_GUARD, useClass: RolesGuard },
    { provide: APP_GUARD, useClass: ThrottlerGuard },
    { provide: APP_INTERCEPTOR, useClass: AuditInterceptor },
  ],
})
export class AppModule {}
