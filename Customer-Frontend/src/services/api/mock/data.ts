import type {
  CandidateProfile,
  CustomerUser,
  InterviewDetail,
  InterviewSession,
  PerformanceSummary,
  ResumeRecord,
} from '../../../types';

export const mockUser: CustomerUser = {
  id: 'cust-1',
  email: 'maya@studio.parakeet.ai',
  firstName: 'Maya',
  lastName: 'Okonkwo',
  role: 'candidate',
  candidate: {
    id: 'cand-maya',
    firstName: 'Maya',
    lastName: 'Okonkwo',
    email: 'maya@studio.parakeet.ai',
    currentTitle: 'Product Engineer',
    currentCompany: 'Northwind',
    skills: ['TypeScript', 'Product thinking', 'Systems'],
    consentGiven: true,
  },
};

export const mockResumes: ResumeRecord[] = [
  {
    id: 'res-1',
    originalFilename: 'Maya_Okonkwo_Resume.pdf',
    sizeBytes: 212_000,
    currentVersion: 3,
    createdAt: new Date(Date.now() - 86400000 * 6).toISOString(),
  },
];

export const mockSessions: InterviewSession[] = [
  {
    id: 'int-1',
    title: 'technical practice',
    jobTitle: 'Staff Engineer',
    company: 'Practice studio',
    interviewType: 'technical',
    status: 'completed',
    createdAt: new Date(Date.now() - 86400000).toISOString(),
  },
  {
    id: 'int-2',
    title: 'hr practice',
    jobTitle: 'Product Engineer',
    company: 'Practice studio',
    interviewType: 'hr',
    status: 'scheduled',
    scheduledAt: new Date(Date.now() + 86400000 * 2).toISOString(),
    createdAt: new Date().toISOString(),
  },
];

export const mockDetail: InterviewDetail = {
  session: mockSessions[0],
  questions: [
    { id: 'q1', text: 'Walk me through a production incident you owned end to end.', sequence: 1 },
    { id: 'q2', text: 'How do you design retries and timeouts for a downstream API?', sequence: 2 },
    { id: 'q3', text: 'Explain a data model you would use for interview transcripts.', sequence: 3 },
    { id: 'q4', text: 'How would you debug a slow React page in production?', sequence: 4 },
    { id: 'q5', text: 'Tell me about a trade-off you made in a recent design.', sequence: 5 },
  ],
  responses: [],
  analytics: [],
};

export const mockPerformance: PerformanceSummary = {
  totals: { interviews: 8, completed: 5, practice: 6 },
  recent: mockSessions,
  latestInsights: [
    {
      kind: 'candidate_response',
      origin: 'ai_generated',
      payload: {
        disclaimer: 'Coaching notes, not a hiring score.',
        insights: [
          { dimension: 'completeness', label: 'Answers tend to include a concrete example' },
          { dimension: 'star_structure', label: 'Some answers resemble STAR-style structure' },
        ],
      },
    },
  ],
  disclaimer:
    'Practice scores are coaching signals, not hiring decisions. AI-generated notes need your own judgment.',
};

export const mockProfile: CandidateProfile = mockUser.candidate!;
