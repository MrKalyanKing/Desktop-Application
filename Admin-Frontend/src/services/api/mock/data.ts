import type {
  AnalyticsRow,
  Candidate,
  DashboardSummary,
  DesktopStatus,
  InterviewSession,
  ReportRecord,
  ResumeRecord,
  TranscriptSegment,
} from '../../../types';

const now = Date.now();

export const mockUser = {
  id: 'user-1',
  email: 'admin@parakeet.ai',
  firstName: 'Ava',
  lastName: 'Chen',
  role: 'owner',
  organization: { id: 'org-1', name: 'Parakeet AI', slug: 'parakeet-demo' },
};

export const mockCandidates: Candidate[] = [
  {
    id: 'cand-1',
    firstName: 'Jordan',
    lastName: 'Lee',
    email: 'jordan.lee@example.com',
    currentTitle: 'Senior Frontend Engineer',
    currentCompany: 'Northwind Labs',
    skills: ['TypeScript', 'React', 'System Design'],
    consentGiven: true,
  },
  {
    id: 'cand-2',
    firstName: 'Samira',
    lastName: 'Khan',
    email: 'samira.khan@example.com',
    currentTitle: 'Staff Backend Engineer',
    currentCompany: 'Helios',
    skills: ['Go', 'Postgres', 'Kafka'],
    consentGiven: true,
  },
];

export const mockResumes: ResumeRecord[] = [
  {
    id: 'res-1',
    originalFilename: 'Jordan_Lee_Resume.pdf',
    mimeType: 'application/pdf',
    sizeBytes: 248_320,
    status: 'uploaded',
    currentVersion: 2,
    extractedJson: { skills: ['TypeScript', 'React'], extraction: 'filename_heuristic' },
    candidate: mockCandidates[0],
    createdAt: new Date(now - 86400000 * 4).toISOString(),
  },
  {
    id: 'res-2',
    originalFilename: 'Samira_Khan_CV.docx',
    mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    sizeBytes: 188_416,
    status: 'uploaded',
    currentVersion: 1,
    candidate: mockCandidates[1],
    createdAt: new Date(now - 86400000 * 2).toISOString(),
  },
];

export const mockSessions: InterviewSession[] = [
  {
    id: 'ses-1',
    title: 'Staff Engineer — Platform',
    jobTitle: 'Staff Engineer',
    company: 'Parakeet AI',
    interviewType: 'technical',
    status: 'completed',
    desktopSyncStatus: 'synced',
    startedAt: new Date(now - 3600000).toISOString(),
    endedAt: new Date(now - 600000).toISOString(),
    consentCaptured: true,
    candidate: mockCandidates[0],
    resume: mockResumes[0],
    createdAt: new Date(now - 86400000).toISOString(),
  },
  {
    id: 'ses-2',
    title: 'Backend Screen — Infra',
    jobTitle: 'Staff Backend Engineer',
    company: 'Parakeet AI',
    interviewType: 'hr',
    status: 'scheduled',
    desktopSyncStatus: 'idle',
    scheduledAt: new Date(now + 86400000).toISOString(),
    consentCaptured: false,
    candidate: mockCandidates[1],
    resume: mockResumes[1],
    createdAt: new Date(now - 3600000).toISOString(),
  },
  {
    id: 'ses-3',
    title: 'Mock interview — behavioral',
    jobTitle: 'Product Engineer',
    company: 'Northwind',
    interviewType: 'mock',
    status: 'in_progress',
    desktopSyncStatus: 'syncing',
    startedAt: new Date(now - 600000).toISOString(),
    consentCaptured: true,
    candidate: mockCandidates[0],
    createdAt: new Date(now - 7200000).toISOString(),
  },
];

export const mockSegments: TranscriptSegment[] = [
  {
    id: 'seg-1',
    sessionId: 'ses-1',
    speakerType: 'interviewer',
    text: 'Walk me through a production incident you owned end to end.',
    startMs: 12000,
    endMs: 18000,
    sequence: 1,
    source: 'desktop_system',
    eventId: 'evt-1',
  },
  {
    id: 'seg-2',
    sessionId: 'ses-1',
    speakerType: 'candidate',
    text: 'Last quarter our checkout p95 spiked. I led the investigation, isolated lock contention, shipped a fix, and wrote the postmortem.',
    startMs: 19000,
    endMs: 48000,
    sequence: 2,
    source: 'desktop_mic',
    eventId: 'evt-2',
  },
  {
    id: 'seg-3',
    sessionId: 'ses-1',
    speakerType: 'copilot',
    text: 'Lead with impact, then the debug path, then the prevention work.',
    startMs: 20000,
    endMs: 26000,
    sequence: 3,
    source: 'desktop_gemini',
    eventId: 'evt-3',
  },
];

export const mockAnalytics: AnalyticsRow[] = [
  {
    id: 'an-1',
    sessionId: 'ses-1',
    kind: 'session_summary',
    origin: 'measured',
    isAiGenerated: false,
    humanReviewStatus: 'accepted',
    payload: {
      candidate: { speakingMs: 29000, wordCount: 28 },
      interviewer: { speakingMs: 6000, questionCount: 1 },
    },
    generatedAt: new Date().toISOString(),
  },
  {
    id: 'an-2',
    sessionId: 'ses-1',
    kind: 'candidate_response',
    origin: 'ai_generated',
    isAiGenerated: true,
    humanReviewStatus: 'pending',
    payload: {
      disclaimer: 'AI-generated assessment for review. Not an objective measure of ability.',
      insights: [
        { dimension: 'completeness', label: 'Generally developed answers', evidenceSequences: [2] },
        { dimension: 'examples', label: 'Uses concrete examples in at least one answer' },
      ],
    },
    generatedAt: new Date().toISOString(),
  },
];

export const mockReports: ReportRecord[] = [
  {
    id: 'rep-1',
    sessionId: 'ses-1',
    templateKey: 'session_summary',
    status: 'ready',
    createdAt: new Date().toISOString(),
  },
];

export const mockDashboard: DashboardSummary = {
  totals: { sessions: 18, completed: 11, upcoming: 4, resumes: 27, candidates: 22 },
  desktop: { connected: true, activeDevices: 1 },
  recentSessions: mockSessions,
  trends: Array.from({ length: 10 }, (_, i) => ({
    day: new Date(now - (9 - i) * 86400000).toISOString(),
    count: 1 + ((i * 3) % 5),
  })),
};

export const mockDesktop: DesktopStatus = {
  connected: true,
  device: {
    id: 'dev-1',
    name: 'KS Vision — Lenovo',
    lastSeenAt: new Date().toISOString(),
    lastSyncAt: new Date(now - 120000).toISOString(),
  },
};
