export type InterviewType = 'mock' | 'technical' | 'hr' | 'general';

export interface CustomerUser {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  role: string;
  candidate?: CandidateProfile;
}

export interface CandidateProfile {
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
  sizeBytes: number;
  currentVersion: number;
  createdAt: string;
}

export interface InterviewSession {
  id: string;
  title: string;
  jobTitle?: string | null;
  company?: string | null;
  interviewType: InterviewType;
  status: string;
  scheduledAt?: string | null;
  createdAt: string;
}

export interface InterviewQuestion {
  id: string;
  text: string;
  sequence: number;
  category?: string | null;
}

export interface InterviewDetail {
  session: InterviewSession;
  questions: InterviewQuestion[];
  responses: { id: string; questionId?: string | null; text: string }[];
  analytics: { id: string; kind: string; origin: string; payload: Record<string, unknown> }[];
}

export interface PerformanceSummary {
  totals: { interviews: number; completed: number; practice: number };
  recent: InterviewSession[];
  latestInsights: { kind: string; origin: string; payload: Record<string, unknown> }[];
  disclaimer: string;
}
