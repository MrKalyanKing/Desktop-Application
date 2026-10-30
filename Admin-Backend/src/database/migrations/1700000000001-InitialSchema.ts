import { MigrationInterface, QueryRunner } from 'typeorm';

export class InitialSchema1700000000001 implements MigrationInterface {
  name = 'InitialSchema1700000000001';

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`CREATE EXTENSION IF NOT EXISTS "pgcrypto"`);
    await queryRunner.query(`
      DO $$ BEGIN
        CREATE TYPE user_role AS ENUM ('owner','admin','interviewer','viewer');
      EXCEPTION WHEN duplicate_object THEN null; END $$;
    `);
    await queryRunner.query(`
      DO $$ BEGIN
        CREATE TYPE user_status AS ENUM ('active','invited','disabled');
      EXCEPTION WHEN duplicate_object THEN null; END $$;
    `);
    await queryRunner.query(`
      DO $$ BEGIN
        CREATE TYPE interview_type AS ENUM ('mock','technical','hr','general');
      EXCEPTION WHEN duplicate_object THEN null; END $$;
    `);
    await queryRunner.query(`
      DO $$ BEGIN
        CREATE TYPE interview_session_status AS ENUM ('draft','scheduled','in_progress','completed','archived');
      EXCEPTION WHEN duplicate_object THEN null; END $$;
    `);
    await queryRunner.query(`
      DO $$ BEGIN
        CREATE TYPE desktop_sync_status AS ENUM ('idle','pending','syncing','synced','error');
      EXCEPTION WHEN duplicate_object THEN null; END $$;
    `);
    await queryRunner.query(`
      DO $$ BEGIN
        CREATE TYPE speaker_type AS ENUM ('candidate','interviewer','copilot','system','unknown');
      EXCEPTION WHEN duplicate_object THEN null; END $$;
    `);
    await queryRunner.query(`
      DO $$ BEGIN
        CREATE TYPE transcript_source AS ENUM ('desktop_mic','desktop_system','desktop_gemini','web','import');
      EXCEPTION WHEN duplicate_object THEN null; END $$;
    `);

    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS organizations (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        name varchar NOT NULL,
        slug varchar NOT NULL UNIQUE,
        settings jsonb NOT NULL DEFAULT '{}',
        "consentRequired" boolean NOT NULL DEFAULT true,
        "retentionDays" int NOT NULL DEFAULT 365,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        "updatedAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS users (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "organizationId" uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
        email varchar NOT NULL UNIQUE,
        "passwordHash" varchar NOT NULL,
        "firstName" varchar NOT NULL,
        "lastName" varchar NOT NULL,
        role user_role NOT NULL DEFAULT 'interviewer',
        status user_status NOT NULL DEFAULT 'active',
        "lastLoginAt" timestamptz,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        "updatedAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`CREATE INDEX IF NOT EXISTS idx_users_org_email ON users ("organizationId", email)`);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS refresh_tokens (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "userId" uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        "tokenHash" varchar NOT NULL UNIQUE,
        "expiresAt" timestamptz NOT NULL,
        "revokedAt" timestamptz,
        "deviceInfo" varchar,
        "createdAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS candidates (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "organizationId" uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
        "firstName" varchar NOT NULL,
        "lastName" varchar NOT NULL,
        email varchar,
        phone varchar,
        "currentTitle" varchar,
        "currentCompany" varchar,
        skills jsonb NOT NULL DEFAULT '[]',
        notes text,
        "consentGiven" boolean NOT NULL DEFAULT false,
        "consentAt" timestamptz,
        "retentionUntil" date,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        "updatedAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS resumes (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "organizationId" uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
        "candidateId" uuid REFERENCES candidates(id) ON DELETE SET NULL,
        "originalFilename" varchar NOT NULL,
        "mimeType" varchar NOT NULL,
        "sizeBytes" int NOT NULL,
        "storageKey" varchar NOT NULL,
        "storageProvider" varchar NOT NULL DEFAULT 'local',
        "extractedText" text,
        "extractedJson" jsonb,
        status varchar NOT NULL DEFAULT 'uploaded',
        "currentVersion" int NOT NULL DEFAULT 1,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        "updatedAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS resume_versions (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "resumeId" uuid NOT NULL REFERENCES resumes(id) ON DELETE CASCADE,
        "versionNumber" int NOT NULL,
        "storageKey" varchar NOT NULL,
        "originalFilename" varchar NOT NULL,
        "extractedText" text,
        "extractedJson" jsonb,
        "createdById" uuid REFERENCES users(id) ON DELETE SET NULL,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        UNIQUE ("resumeId", "versionNumber")
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS interview_sessions (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "organizationId" uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
        "candidateId" uuid REFERENCES candidates(id) ON DELETE SET NULL,
        "resumeId" uuid REFERENCES resumes(id) ON DELETE SET NULL,
        "createdById" uuid NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
        title varchar NOT NULL,
        "jobTitle" varchar,
        company varchar,
        "interviewType" interview_type NOT NULL DEFAULT 'general',
        status interview_session_status NOT NULL DEFAULT 'draft',
        "desktopSyncStatus" desktop_sync_status NOT NULL DEFAULT 'idle',
        "scheduledAt" timestamptz,
        "startedAt" timestamptz,
        "endedAt" timestamptz,
        "consentCaptured" boolean NOT NULL DEFAULT false,
        notes text,
        "lastTranscriptSequence" int NOT NULL DEFAULT 0,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        "updatedAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS interview_questions (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "sessionId" uuid NOT NULL REFERENCES interview_sessions(id) ON DELETE CASCADE,
        "askedBy" varchar NOT NULL DEFAULT 'interviewer',
        text text NOT NULL,
        category varchar,
        sequence int NOT NULL,
        "askedAt" timestamptz,
        "eventId" varchar,
        "createdAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS interview_responses (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "sessionId" uuid NOT NULL REFERENCES interview_sessions(id) ON DELETE CASCADE,
        "questionId" uuid REFERENCES interview_questions(id) ON DELETE SET NULL,
        speaker speaker_type NOT NULL DEFAULT 'candidate',
        text text NOT NULL,
        "startMs" int,
        "endMs" int,
        "durationMs" int,
        "eventId" varchar,
        "createdAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS transcript_segments (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "sessionId" uuid NOT NULL REFERENCES interview_sessions(id) ON DELETE CASCADE,
        "speakerType" speaker_type NOT NULL DEFAULT 'unknown',
        text text NOT NULL,
        "startMs" int NOT NULL DEFAULT 0,
        "endMs" int NOT NULL DEFAULT 0,
        sequence int NOT NULL,
        source transcript_source NOT NULL DEFAULT 'desktop_gemini',
        confidence float,
        "eventId" varchar NOT NULL,
        metadata jsonb,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        UNIQUE ("sessionId", "eventId")
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS interview_events (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "sessionId" uuid NOT NULL REFERENCES interview_sessions(id) ON DELETE CASCADE,
        "eventType" varchar NOT NULL,
        "eventId" varchar NOT NULL,
        payload jsonb NOT NULL DEFAULT '{}',
        "occurredAt" timestamptz NOT NULL,
        "receivedAt" timestamptz NOT NULL DEFAULT now(),
        UNIQUE ("sessionId", "eventId")
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS interview_analytics (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "sessionId" uuid NOT NULL REFERENCES interview_sessions(id) ON DELETE CASCADE,
        kind varchar NOT NULL,
        origin varchar NOT NULL DEFAULT 'ai_generated',
        provider varchar NOT NULL DEFAULT 'heuristic',
        model varchar,
        payload jsonb NOT NULL,
        "isAiGenerated" boolean NOT NULL DEFAULT true,
        "humanReviewStatus" varchar NOT NULL DEFAULT 'pending',
        "humanReviewNotes" text,
        "reviewedById" uuid,
        "generatedAt" timestamptz NOT NULL DEFAULT now(),
        "updatedAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS interview_reports (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "sessionId" uuid NOT NULL REFERENCES interview_sessions(id) ON DELETE CASCADE,
        "templateKey" varchar NOT NULL DEFAULT 'session_summary',
        status varchar NOT NULL DEFAULT 'pending',
        "storageKey" varchar,
        "mimeType" varchar,
        error text,
        "createdById" uuid REFERENCES users(id) ON DELETE SET NULL,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        "generatedAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS desktop_devices (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "organizationId" uuid NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
        "userId" uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        "deviceName" varchar NOT NULL,
        "deviceFingerprint" varchar NOT NULL,
        "tokenHash" varchar,
        status varchar NOT NULL DEFAULT 'active',
        "lastSeenAt" timestamptz,
        "lastSyncAt" timestamptz,
        "pendingCount" int NOT NULL DEFAULT 0,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        "updatedAt" timestamptz NOT NULL DEFAULT now(),
        UNIQUE ("organizationId", "deviceFingerprint")
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS sync_records (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "deviceId" uuid NOT NULL REFERENCES desktop_devices(id) ON DELETE CASCADE,
        "sessionId" uuid NOT NULL REFERENCES interview_sessions(id) ON DELETE CASCADE,
        "lastSequence" int NOT NULL DEFAULT 0,
        "pendingCount" int NOT NULL DEFAULT 0,
        "lastError" text,
        "lastSyncedAt" timestamptz,
        "createdAt" timestamptz NOT NULL DEFAULT now(),
        "updatedAt" timestamptz NOT NULL DEFAULT now(),
        UNIQUE ("deviceId", "sessionId")
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS audit_logs (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "organizationId" uuid NOT NULL,
        "userId" uuid,
        action varchar NOT NULL,
        "resourceType" varchar NOT NULL,
        "resourceId" varchar,
        ip varchar,
        "userAgent" varchar,
        metadata jsonb,
        "createdAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
    await queryRunner.query(`
      CREATE TABLE IF NOT EXISTS notifications (
        id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
        "organizationId" uuid NOT NULL,
        "userId" uuid NOT NULL,
        type varchar NOT NULL,
        title varchar NOT NULL,
        body text NOT NULL,
        data jsonb,
        "readAt" timestamptz,
        "createdAt" timestamptz NOT NULL DEFAULT now()
      )
    `);
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    const tables = [
      'notifications','audit_logs','sync_records','desktop_devices','interview_reports',
      'interview_analytics','interview_events','transcript_segments','interview_responses',
      'interview_questions','interview_sessions','resume_versions','resumes','candidates',
      'refresh_tokens','users','organizations',
    ];
    for (const table of tables) {
      await queryRunner.query(`DROP TABLE IF EXISTS ${table} CASCADE`);
    }
  }
}
