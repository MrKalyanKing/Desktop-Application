export enum UserRole {
  OWNER = 'owner',
  ADMIN = 'admin',
  INTERVIEWER = 'interviewer',
  VIEWER = 'viewer',
  CANDIDATE = 'candidate',
}

export enum UserStatus {
  ACTIVE = 'active',
  INVITED = 'invited',
  DISABLED = 'disabled',
}

export enum InterviewType {
  MOCK = 'mock',
  TECHNICAL = 'technical',
  HR = 'hr',
  GENERAL = 'general',
}

export enum InterviewSessionStatus {
  DRAFT = 'draft',
  SCHEDULED = 'scheduled',
  IN_PROGRESS = 'in_progress',
  COMPLETED = 'completed',
  ARCHIVED = 'archived',
}

export enum DesktopSyncStatus {
  IDLE = 'idle',
  PENDING = 'pending',
  SYNCING = 'syncing',
  SYNCED = 'synced',
  ERROR = 'error',
}

export enum SpeakerType {
  CANDIDATE = 'candidate',
  INTERVIEWER = 'interviewer',
  COPILOT = 'copilot',
  SYSTEM = 'system',
  UNKNOWN = 'unknown',
}

export enum TranscriptSource {
  DESKTOP_MIC = 'desktop_mic',
  DESKTOP_SYSTEM = 'desktop_system',
  DESKTOP_GEMINI = 'desktop_gemini',
  WEB = 'web',
  IMPORT = 'import',
}

export enum QuestionAskedBy {
  INTERVIEWER = 'interviewer',
  CANDIDATE = 'candidate',
  SYSTEM = 'system',
  COPILOT = 'copilot',
}

export enum AnalyticsKind {
  CANDIDATE_RESPONSE = 'candidate_response',
  INTERVIEWER = 'interviewer',
  SESSION_SUMMARY = 'session_summary',
}

export enum AnalyticsOrigin {
  MEASURED = 'measured',
  AI_GENERATED = 'ai_generated',
}

export enum HumanReviewStatus {
  PENDING = 'pending',
  ACCEPTED = 'accepted',
  REJECTED = 'rejected',
  EDITED = 'edited',
}

export enum ReportStatus {
  PENDING = 'pending',
  GENERATING = 'generating',
  READY = 'ready',
  FAILED = 'failed',
}

export enum DeviceStatus {
  ACTIVE = 'active',
  REVOKED = 'revoked',
}

export enum InterviewEventType {
  SESSION_STARTED = 'session_started',
  SESSION_ENDED = 'session_ended',
  QUESTION = 'question',
  TRANSCRIPT_SEGMENT = 'transcript_segment',
  RESPONSE = 'response',
  STATUS = 'status',
  HEARTBEAT = 'heartbeat',
  ANALYTICS_READY = 'analytics_ready',
}
