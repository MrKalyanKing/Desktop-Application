import { NestFactory } from '@nestjs/core';
import * as bcrypt from 'bcryptjs';
import { AppModule } from '../../app.module';
import { Organization } from '../../modules/organizations/organization.entity';
import { User } from '../../modules/users/user.entity';
import { Candidate } from '../../modules/candidates/candidate.entity';
import { InterviewSession } from '../../modules/interview-sessions/interview-session.entity';
import { TranscriptSegment } from '../../modules/transcription/transcript-segment.entity';
import { Notification } from '../../modules/notifications/notification.entity';
import {
  InterviewSessionStatus,
  InterviewType,
  SpeakerType,
  TranscriptSource,
  UserRole,
  UserStatus,
} from '../../common/enums';
import { DataSource } from 'typeorm';

async function run() {
  const app = await NestFactory.createApplicationContext(AppModule);
  const dataSource = app.get(DataSource);

  let org = await dataSource.getRepository(Organization).findOne({ where: { slug: 'parakeet-demo' } });
  if (!org) {
    org = await dataSource.getRepository(Organization).save({
      name: 'Parakeet AI',
      slug: 'parakeet-demo',
      settings: { product: 'interview-intelligence' },
    });
  }

  let user = await dataSource.getRepository(User).findOne({ where: { email: 'admin@parakeet.ai' } });
  if (!user) {
    user = await dataSource.getRepository(User).save({
      organizationId: org.id,
      email: 'admin@parakeet.ai',
      passwordHash: await bcrypt.hash('Admin123!', 12),
      firstName: 'Ava',
      lastName: 'Chen',
      role: UserRole.OWNER,
      status: UserStatus.ACTIVE,
    });
  }

  let candidate = await dataSource.getRepository(Candidate).findOne({
    where: { organizationId: org.id, email: 'jordan.lee@example.com' },
  });
  if (!candidate) {
    candidate = await dataSource.getRepository(Candidate).save({
      organizationId: org.id,
      firstName: 'Jordan',
      lastName: 'Lee',
      email: 'jordan.lee@example.com',
      currentTitle: 'Senior Frontend Engineer',
      currentCompany: 'Northwind Labs',
      skills: ['TypeScript', 'React', 'System Design'],
      consentGiven: true,
      consentAt: new Date(),
    });
  }

  let session = await dataSource.getRepository(InterviewSession).findOne({
    where: { organizationId: org.id, title: 'Staff Engineer — Platform' },
  });
  if (!session) {
    session = await dataSource.getRepository(InterviewSession).save({
      organizationId: org.id,
      createdById: user.id,
      candidateId: candidate.id,
      title: 'Staff Engineer — Platform',
      jobTitle: 'Staff Engineer',
      company: 'Parakeet AI',
      interviewType: InterviewType.TECHNICAL,
      status: InterviewSessionStatus.COMPLETED,
      consentCaptured: true,
      startedAt: new Date(Date.now() - 3600_000),
      endedAt: new Date(),
    });

    await dataSource.getRepository(TranscriptSegment).save([
      {
        sessionId: session.id,
        speakerType: SpeakerType.INTERVIEWER,
        text: 'Walk me through a production incident you owned end to end.',
        startMs: 12_000,
        endMs: 18_000,
        sequence: 1,
        source: TranscriptSource.DESKTOP_SYSTEM,
        eventId: 'seed-event-1',
      },
      {
        sessionId: session.id,
        speakerType: SpeakerType.CANDIDATE,
        text: 'Last quarter our checkout p95 spiked. I led the investigation, isolated a lock contention issue, shipped a fix, and wrote the postmortem.',
        startMs: 19_000,
        endMs: 48_000,
        sequence: 2,
        source: TranscriptSource.DESKTOP_MIC,
        eventId: 'seed-event-2',
      },
    ]);
  }

  await dataSource.getRepository(Notification).save({
    organizationId: org.id,
    userId: user.id,
    type: 'welcome',
    title: 'Parakeet Admin is ready',
    body: 'Connect the desktop assistant with a device token to stream interview transcripts here.',
  });

  console.log('Seed complete. Login: admin@parakeet.ai / Admin123!');
  await app.close();
}

run().catch((error) => {
  console.error(error);
  process.exit(1);
});
