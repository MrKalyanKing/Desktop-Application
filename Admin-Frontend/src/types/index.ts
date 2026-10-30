export type InterviewType = 'mock' | 'technical' | 'hr' | 'general';
export type SessionStatus = 'draft' | 'scheduled' | 'in_progress' | 'completed' | 'archived';
export type SyncStatus = 'idle' | 'pending' | 'syncing' | 'synced' | 'error';
export type SpeakerType = 'candidate' | 'interviewer' | 'copilot' | 'system' | 'unknown';
export type AnalyticsOrigin = 'measured' | 'ai_generated';
export type ReviewStatus = 'pending' | 'accepted' | 'rejected' | 'edited';

export interface AuthUser {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  role: string;
  organizationId?: string;
  organization?: { id: string; name: string; slug: string };
}

export interface Candidate {
  id: string;
  firstName: string;
  lastName: string;
  email?: string | null;
  currentTitle?: string | null;
  currentCompany?: string | null;
  skills: string[];
  consentGiven: boolean;
}

export interface ResumeRecord {
  id: string;
  originalFilename: string;
  mimeType: string;
  sizeBytes: number;
  status: string;
  currentVersion: number;
  extractedJson?: Record<string, unknown> | null;
  candidate?: Candidate | null;
  createdAt: string;
}

export interface InterviewSession {
  id: string;
  title: string;
  jobTitle?: string | null;
  company?: string | null;
  interviewType: InterviewType;
  status: SessionStatus;
  desktopSyncStatus: SyncStatus;
  scheduledAt?: string | null;
  startedAt?: string | null;
  endedAt?: string | null;
  consentCaptured: boolean;
  candidate?: Candidate | null;
  resume?: ResumeRecord | null;
  createdAt: string;
}

export interface TranscriptSegment {
  id: string;
  sessionId: string;
  speakerType: SpeakerType;
  text: string;
  startMs: number;
  endMs: number;
  sequence: number;
  source: string;
  eventId: string;
}

export interface AnalyticsRow {
  id: string;
  sessionId: string;
  kind: string;
  origin: AnalyticsOrigin;
  isAiGenerated: boolean;
  humanReviewStatus: ReviewStatus;
  payload: Record<string, unknown>;
  generatedAt: string;
}

export interface ReportRecord {
  id: string;
  sessionId: string;
  templateKey: string;
  status: string;
  createdAt: string;
}

export interface DashboardSummary {
  totals: {
    sessions: number;
    completed: number;
    upcoming: number;
    resumes: number;
    candidates: number;
  };
  desktop: { connected: boolean; activeDevices: number };
  recentSessions: InterviewSession[];
  trends: { day: string; count: string | number }[];
}

export interface DesktopStatus {
  connected: boolean;
  device: { id: string; name: string; lastSeenAt?: string | null; lastSyncAt?: string | null } | null;
}
