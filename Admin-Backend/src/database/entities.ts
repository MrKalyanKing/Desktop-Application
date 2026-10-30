import { Organization } from '../modules/organizations/organization.entity';
import { User } from '../modules/users/user.entity';
import { RefreshToken } from '../modules/auth/refresh-token.entity';
import { Candidate } from '../modules/candidates/candidate.entity';
import { Resume } from '../modules/resumes/resume.entity';
import { ResumeVersion } from '../modules/resumes/resume-version.entity';
import { InterviewSession } from '../modules/interview-sessions/interview-session.entity';
import { InterviewQuestion } from '../modules/interview-questions/interview-question.entity';
import { InterviewResponse } from '../modules/interview-responses/interview-response.entity';
import { TranscriptSegment } from '../modules/transcription/transcript-segment.entity';
import { InterviewEvent } from '../modules/transcription/interview-event.entity';
import { InterviewAnalytics } from '../modules/interview-analytics/interview-analytics.entity';
import { InterviewReport } from '../modules/reports/interview-report.entity';
import { DesktopDevice } from '../modules/desktop-integration/desktop-device.entity';
import { SyncRecord } from '../modules/desktop-integration/sync-record.entity';
import { AuditLog } from '../modules/audit-logs/audit-log.entity';
import { Notification } from '../modules/notifications/notification.entity';

export const ALL_ENTITIES = [
  Organization,
  User,
  RefreshToken,
  Candidate,
  Resume,
  ResumeVersion,
  InterviewSession,
  InterviewQuestion,
  InterviewResponse,
  TranscriptSegment,
  InterviewEvent,
  InterviewAnalytics,
  InterviewReport,
  DesktopDevice,
  SyncRecord,
  AuditLog,
  Notification,
];
