# Parakeet AI — Interview Intelligence Platform

The existing **KS Vision** Tauri desktop assistant is unchanged as the real-time interview client.  
`Admin-Backend` (NestJS) and `Admin-Frontend` (React) are a separate web product that stores sessions, resumes, transcripts, analytics, and reports.

```
KS Vision (Tauri + React + Rust)
  mic / loopback / Gemini voice / local SQLite
              |
              | HTTPS REST  /api/v1/*
              | Bearer JWT (desktop device token)
              v
     Admin-Backend (NestJS)
              |
     +--------+--------+
     |        |        |
  Postgres   S3/local  Analytics / Reports
              ^
              |
     Admin-Frontend (recruiters)   Customer-Frontend (candidates, port 5175)

```

## What the desktop app already does

Inspected `ks-vision` and `ks-vision-backend`:

| Capability | Where it lives | Cloud role |
|---|---|---|
| Microphone + system loopback | Rust `audio` | Stays on desktop |
| VAD / RNNoise / AGC | Rust audio pipeline | Stays on desktop |
| Gemini multimodal voice | `transcription/gemini_voice.rs` | Stays on desktop |
| Interviewer question extract | `voice-system-question` event | Synced as interviewer transcript |
| Copilot answers | `voice-gemini-answer` event | Synced as copilot speaker, not candidate speech |
| Local history | SQLite `conversation_history` | Unchanged |
| Screenshots / OCR / stealth window | Rust modules | Unchanged |

Important: the desktop app does **not** currently produce candidate STT. Audio goes straight to Gemini. The NestJS transcript model still accepts `candidate` segments so true speech-to-text can be added later without a schema rewrite.

## Integration contract (Rust → NestJS)

The desktop app **never** receives `DATABASE_URL` or AWS keys.

1. `POST /api/v1/auth/desktop/register` — email/password + device fingerprint → JWT with `deviceId`.
2. `GET /api/v1/desktop/sessions` — assigned/scheduled/in-progress sessions + resume metadata.
3. `POST /api/v1/interview-sessions/:id/start` — register an active session.
4. `POST /api/v1/desktop/sync/:sessionId` — incremental `segments[]` + `events[]`.
5. Each record has `eventId`. Duplicate `(sessionId, eventId)` is ignored.
6. `GET /api/v1/interview-sessions/:id/analytics` — server-generated insights.
7. Offline: Rust writes `cloud_sync_queue.json` and flushes later.

New Tauri commands (opt-in, default **disabled**):

- `cloud_configure` `cloud_login` `cloud_logout` `cloud_set_session` `cloud_get_status` `cloud_flush_queue`

Voice events enqueue only when cloud sync is enabled **and** an active session id is set.

## API (versioned)

Base: `/api/v1`  
Swagger: `http://localhost:3001/api/v1/docs`

- Auth: `/auth/register` `/auth/login` `/auth/refresh` `/auth/logout` `/auth/me` `/auth/desktop/register`
- Users / orgs: `/users` `/organizations/current`
- Candidates / resumes: `/candidates` `/resumes`
- Sessions: `/interview-sessions` `/:id/start` `/:id/complete` `/:id/archive`
- Live interview: `/:id/questions` `/:id/responses` `/:id/transcript` `/:id/events` `/:id/analytics`
- Desktop: `/desktop/heartbeat` `/desktop/sessions` `/desktop/sync` `/desktop/sync/:sessionId`
- Reports: `/reports` `/interview-sessions/:id/reports`
- Dashboard / audit: `/dashboard/summary` `/audit-logs` `/health`

## Database (PostgreSQL + TypeORM)

Core entities: Organization, User, RefreshToken, Candidate, Resume, ResumeVersion, InterviewSession, InterviewQuestion, InterviewResponse, TranscriptSegment, InterviewEvent, InterviewAnalytics, InterviewReport, DesktopDevice, SyncRecord, AuditLog, Notification.

Org isolation is enforced on every query via the JWT `orgId`.

```
Organization 1──* User
Organization 1──* Candidate 1──* Resume 1──* ResumeVersion
Candidate 1──* InterviewSession *──1 Resume
InterviewSession 1──* TranscriptSegment (unique sessionId+eventId)
InterviewSession 1──* InterviewQuestion / Response / Event / Analytics / Report
DesktopDevice 1──* SyncRecord *──1 InterviewSession
```

## Local setup

**Backend**

```bash
cd Admin-Backend
copy .env.example .env
docker compose up -d
npm install
npm run start:dev
npm run seed
```

Login: `admin@parakeet.ai` / `Admin123!`

**Admin dashboard**

```bash
cd Admin-Frontend
copy .env.example .env
npm install
npm run dev
```

Admin UI: `http://localhost:5174` with `VITE_USE_MOCK=true`.

**Customer website (Parakeet Studio)**

```bash
cd Customer-Frontend
copy .env.example .env
npm install
npm run dev
```

Candidate site: `http://localhost:5175` — landing, resume upload, browser practice interviews, personal performance.

Portal API: `/api/v1/portal/auth/register`, `/portal/resumes`, `/portal/interviews/practice`, `/portal/performance`.
Customer registration creates a personal workspace with role `candidate`.


`VITE_USE_MOCK=true` (default) renders the liquid-glass UI from `src/services/api/mock`.  
Set `VITE_USE_MOCK=false` to hit NestJS.

**Desktop cloud sync (optional)**

Call `cloud_configure` with `http://localhost:3001/api/v1`, then `cloud_login`, then `cloud_set_session`. Capture/Gemini keep running if the API is down.

## Production notes

- Set `TYPEORM_SYNC=false` and run `npm run migration:run`.
- Set `STORAGE_PROVIDER=s3` and AWS credentials **only** on the NestJS host.
- Rotate `JWT_ACCESS_SECRET` / `JWT_REFRESH_SECRET`.
- Rate limits are global plus stricter auth throttles.
- Resume MIME/size/PDF magic-byte checks are server-side.
- Refresh tokens live in tab `sessionStorage` on the web client, not `localStorage`.
- Analytics payloads that are AI-generated stay in `pending` review until a human accepts them.

## Tests

```bash
cd Admin-Backend
npm test                 # transcript idempotency unit test
npm run test:e2e         # live postgres contract tests if DATABASE_URL is set
```
