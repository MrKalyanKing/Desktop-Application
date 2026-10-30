import axios from 'axios';
import { API_BASE, USE_MOCK } from '../../lib/config';
import type {
  CustomerUser,
  InterviewDetail,
  InterviewSession,
  InterviewType,
  PerformanceSummary,
  ResumeRecord,
} from '../../types';
import {
  mockDetail,
  mockPerformance,
  mockResumes,
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
  async register(body: { email: string; password: string; firstName: string; lastName: string }) {
    if (USE_MOCK) {
      setAccessToken('mock');
      return { accessToken: 'mock', user: { ...mockUser, ...body } };
    }
    const { data } = await http.post('/portal/auth/register', body);
    setAccessToken(data.accessToken);
    sessionStorage.setItem('parakeet.customer.refresh', data.refreshToken);
    return data;
  },
  async login(email: string, password: string) {
    if (USE_MOCK) {
      setAccessToken('mock');
      return { accessToken: 'mock', user: mockUser };
    }
    const { data } = await http.post('/portal/auth/login', { email, password });
    setAccessToken(data.accessToken);
    sessionStorage.setItem('parakeet.customer.refresh', data.refreshToken);
    return data;
  },
  logout() {
    setAccessToken(null);
    sessionStorage.removeItem('parakeet.customer.refresh');
  },
  me(): Promise<CustomerUser> {
    return live(async () => (await http.get('/portal/me')).data, mockUser);
  },
  updateProfile(body: Record<string, unknown>) {
    return live(async () => (await http.patch('/portal/me', body)).data, mockUser.candidate);
  },
  resumes(): Promise<ResumeRecord[]> {
    return live(async () => (await http.get('/portal/resumes')).data, mockResumes);
  },
  uploadResume(file: File) {
    if (USE_MOCK) {
      return Promise.resolve({
        id: `res-${Date.now()}`,
        originalFilename: file.name,
        sizeBytes: file.size,
        currentVersion: 1,
        createdAt: new Date().toISOString(),
      } satisfies ResumeRecord);
    }
    const form = new FormData();
    form.append('file', file);
    return http.post('/portal/resumes', form).then((r) => r.data);
  },
  interviews(): Promise<InterviewSession[]> {
    return live(async () => (await http.get('/portal/interviews')).data, mockSessions);
  },
  interview(id: string): Promise<InterviewDetail> {
    return live(async () => (await http.get(`/portal/interviews/${id}`)).data, {
      ...mockDetail,
      session: { ...mockDetail.session, id },
    });
  },
  startPractice(body: { interviewType?: InterviewType; jobTitle?: string; company?: string; resumeId?: string }) {
    return live(async () => (await http.post('/portal/interviews/practice', body)).data, mockDetail);
  },
  answer(id: string, questionId: string, text: string) {
    return live(
      async () => (await http.post(`/portal/interviews/${id}/answers`, { questionId, text })).data,
      { eventId: `evt-${Date.now()}`, text },
    );
  },
  complete(id: string) {
    return live(async () => (await http.post(`/portal/interviews/${id}/complete`)).data, {
      session: { ...mockSessions[0], status: 'completed' },
      insights: mockPerformance.latestInsights,
    });
  },
  performance(): Promise<PerformanceSummary> {
    return live(async () => (await http.get('/portal/performance')).data, mockPerformance);
  },
};
