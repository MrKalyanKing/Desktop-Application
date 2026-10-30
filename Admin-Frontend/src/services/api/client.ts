import axios from 'axios';
import { API_BASE, USE_MOCK } from '../../lib/config';
import type {
  AnalyticsRow,
  AuthUser,
  Candidate,
  DashboardSummary,
  DesktopStatus,
  InterviewSession,
  ReportRecord,
  ResumeRecord,
  TranscriptSegment,
} from '../../types';
import {
  mockAnalytics,
  mockCandidates,
  mockDashboard,
  mockDesktop,
  mockReports,
  mockResumes,
  mockSegments,
  mockSessions,
  mockUser,
} from './mock/data';

let accessToken: string | null = null;

export function setAccessToken(token: string | null) {
  accessToken = token;
}

const http = axios.create({ baseURL: API_BASE });

http.interceptors.request.use((config) => {
  if (accessToken) config.headers.Authorization = `Bearer ${accessToken}`;
  return config;
});

async function live<T>(fn: () => Promise<T>, fallback: T): Promise<T> {
  if (USE_MOCK) return fallback;
  return fn();
}

export const api = {
  async login(email: string, password: string) {
    if (USE_MOCK) {
      setAccessToken('mock-token');
      sessionStorage.setItem('parakeet.refresh', 'mock-refresh');
      return { accessToken: 'mock-token', user: mockUser };
    }
    const { data } = await http.post('/auth/login', { email, password });
    setAccessToken(data.accessToken);
    sessionStorage.setItem('parakeet.refresh', data.refreshToken);
    return data;
  },
  async me(): Promise<AuthUser> {
    return live(async () => (await http.get('/auth/me')).data, mockUser);
  },
  logout() {
    setAccessToken(null);
    sessionStorage.removeItem('parakeet.refresh');
  },
  dashboard(): Promise<DashboardSummary> {
    return live(async () => (await http.get('/dashboard/summary')).data, mockDashboard);
  },
  sessions(params?: Record<string, string>): Promise<InterviewSession[]> {
    return live(async () => (await http.get('/interview-sessions', { params })).data, mockSessions);
  },
  session(id: string): Promise<InterviewSession> {
    return live(
      async () => (await http.get(`/interview-sessions/${id}`)).data,
      mockSessions.find((s) => s.id === id) ?? mockSessions[0],
    );
  },
  createSession(body: Partial<InterviewSession>) {
    return live(async () => (await http.post('/interview-sessions', body)).data, {
      ...mockSessions[0],
      ...body,
      id: `ses-${Date.now()}`,
    });
  },
  archiveSession(id: string) {
    return live(async () => (await http.post(`/interview-sessions/${id}/archive`)).data, { success: true });
  },
  deleteSession(id: string) {
    return live(async () => (await http.delete(`/interview-sessions/${id}`)).data, { success: true });
  },
  candidates(): Promise<Candidate[]> {
    return live(async () => (await http.get('/candidates')).data, mockCandidates);
  },
  resumes(): Promise<ResumeRecord[]> {
    return live(async () => (await http.get('/resumes')).data, mockResumes);
  },
  uploadResume(file: File, candidateId?: string) {
    if (USE_MOCK) {
      return Promise.resolve({
        id: `res-${Date.now()}`,
        originalFilename: file.name,
        mimeType: file.type,
        sizeBytes: file.size,
        status: 'uploaded',
        currentVersion: 1,
        createdAt: new Date().toISOString(),
      } satisfies ResumeRecord);
    }
    const form = new FormData();
    form.append('file', file);
    if (candidateId) form.append('candidateId', candidateId);
    return http.post('/resumes', form).then((r) => r.data);
  },
  transcript(id: string): Promise<TranscriptSegment[]> {
    return live(async () => (await http.get(`/interview-sessions/${id}/transcript`)).data, mockSegments);
  },
  analytics(id: string): Promise<AnalyticsRow[]> {
    return live(async () => (await http.get(`/interview-sessions/${id}/analytics`)).data, mockAnalytics);
  },
  generateAnalytics(id: string) {
    return live(async () => (await http.post(`/interview-sessions/${id}/analytics/generate`)).data, mockAnalytics);
  },
  reviewAnalytics(id: string, status: string, notes?: string) {
    return live(async () => (await http.patch(`/analytics/${id}/review`, { status, notes })).data, mockAnalytics[1]);
  },
  reports(): Promise<ReportRecord[]> {
    return live(async () => (await http.get('/reports')).data, mockReports);
  },
  generateReport(sessionId: string, templateKey?: string) {
    return live(
      async () => (await http.post(`/interview-sessions/${sessionId}/reports`, { templateKey })).data,
      mockReports[0],
    );
  },
  desktopStatus(): Promise<DesktopStatus> {
    return live(async () => (await http.get('/desktop/sync')).data, mockDesktop);
  },
};
