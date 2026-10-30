import { Injectable, ForbiddenException, NotFoundException } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { randomUUID } from 'crypto';
import { Repository } from 'typeorm';
import {
  InterviewType,
  QuestionAskedBy,
  SpeakerType,
  TranscriptSource,
  UserRole,
} from '../../common/enums';
import type { AuthUser } from '../../common/interfaces/auth-user.interface';
import { AuthService } from '../auth/auth.service';
import { Candidate } from '../candidates/candidate.entity';
import { InterviewAnalyticsService } from '../interview-analytics/interview-analytics.service';
import { InterviewSessionsService } from '../interview-sessions/interview-sessions.service';
import { ResumesService } from '../resumes/resumes.service';
import { TranscriptionService } from '../transcription/transcription.service';

const QUESTION_BANK: Record<InterviewType, string[]> = {
  [InterviewType.MOCK]: [
    'Introduce yourself in about one minute.',
    'Walk me through a project you are proud of.',
    'What is a skill you are actively improving, and how?',
    'Describe a time you had to learn something quickly.',
    'What kind of role are you targeting next?',
  ],
  [InterviewType.TECHNICAL]: [
    'Walk me through a production incident you owned end to end.',
    'How do you design retries and timeouts for a downstream API?',
    'Explain a data model you would use for interview transcripts.',
    'How would you debug a slow React page in production?',
    'Tell me about a trade-off you made in a recent design.',
  ],
  [InterviewType.HR]: [
    'Tell me about yourself.',
    'Why are you interested in this kind of role?',
    'Describe a conflict with a teammate and how you handled it.',
    'What does good feedback look like to you?',
    'Where do you want to grow in the next year?',
  ],
  [InterviewType.GENERAL]: [
    'What drew you to this opportunity?',
    'How do you prioritize when everything feels urgent?',
    'Share an example of communicating a complex idea simply.',
    'What does a strong working relationship with a manager look like?',
    'What questions would you ask your interviewer?',
  ],
};

@Injectable()
export class PortalService {
  constructor(
    private readonly auth: AuthService,
    @InjectRepository(Candidate) private readonly candidates: Repository<Candidate>,
    private readonly resumes: ResumesService,
    private readonly sessions: InterviewSessionsService,
    private readonly transcription: TranscriptionService,
    private readonly analytics: InterviewAnalyticsService,
  ) {}

  register(dto: { email: string; password: string; firstName: string; lastName: string }) {
    return this.auth.registerCustomer(dto);
  }

  login(dto: { email: string; password: string }) {
    return this.auth.login(dto);
  }

  async me(user: AuthUser) {
    this.assertCustomer(user);
    const profile = await this.ensureCandidate(user);
    const auth = await this.auth.me(user);
    return { ...auth, candidate: profile };
  }

  async updateProfile(
    user: AuthUser,
    patch: Partial<Pick<Candidate, 'currentTitle' | 'currentCompany' | 'phone' | 'skills' | 'consentGiven'>>,
  ) {
    const candidate = await this.ensureCandidate(user);
    Object.assign(candidate, patch);
    if (patch.consentGiven) candidate.consentAt = new Date();
    return this.candidates.save(candidate);
  }

  async resumesFor(user: AuthUser) {
    const candidate = await this.ensureCandidate(user);
    const all = await this.resumes.list(user.organizationId);
    return all.filter((resume) => resume.candidateId === candidate.id);
  }

  async uploadResume(user: AuthUser, file: Express.Multer.File) {
    const candidate = await this.ensureCandidate(user);
    return this.resumes.upload(user.organizationId, user.id, file, candidate.id);
  }

  async interviews(user: AuthUser) {
    const candidate = await this.ensureCandidate(user);
    return this.sessions.list(user.organizationId, { candidateId: candidate.id });
  }

  async interview(user: AuthUser, id: string) {
    const candidate = await this.ensureCandidate(user);
    const session = await this.sessions.get(user.organizationId, id);
    if (session.candidateId !== candidate.id) throw new NotFoundException('Interview not found');
    const [questions, responses, segments, insights] = await Promise.all([
      this.transcription.listQuestions(id),
      this.transcription.listResponses(id),
      this.transcription.listSegments(id),
      this.analytics.list(id),
    ]);
    return { session, questions, responses, transcript: segments, analytics: insights };
  }

  async startPractice(
    user: AuthUser,
    input: { interviewType?: InterviewType; jobTitle?: string; company?: string; resumeId?: string },
  ) {
    const candidate = await this.ensureCandidate(user);
    const type = input.interviewType ?? InterviewType.MOCK;
    const session = await this.sessions.create(user.organizationId, user.id, {
      title: `${type} practice`,
      jobTitle: input.jobTitle,
      company: input.company ?? 'Practice studio',
      interviewType: type,
      candidateId: candidate.id,
      resumeId: input.resumeId,
      consentCaptured: candidate.consentGiven,
    });
    await this.sessions.start(user.organizationId, session.id);
    const questions = QUESTION_BANK[type];
    for (const text of questions) {
      await this.transcription.addQuestion(user.organizationId, session.id, {
        text,
        askedBy: QuestionAskedBy.SYSTEM,
        category: type,
      });
    }
    return this.interview(user, session.id);
  }

  async answer(user: AuthUser, sessionId: string, questionId: string, text: string) {
    await this.interview(user, sessionId);
    const eventId = randomUUID();
    await this.transcription.ingestSegments(user.organizationId, sessionId, [
      {
        eventId,
        speakerType: SpeakerType.CANDIDATE,
        text,
        source: TranscriptSource.WEB,
        questionId,
      },
    ]);
    return { eventId, text };
  }

  async complete(user: AuthUser, sessionId: string) {
    await this.interview(user, sessionId);
    const session = await this.sessions.complete(user.organizationId, sessionId);
    const insights = await this.analytics.generate(user.organizationId, sessionId);
    return { session, insights };
  }

  async performance(user: AuthUser) {
    const interviews = await this.interviews(user);
    const completed = interviews.filter((row) => row.status === 'completed');
    const latest = completed[0];
    const latestInsights = latest ? await this.analytics.list(latest.id) : [];
    return {
      totals: {
        interviews: interviews.length,
        completed: completed.length,
        practice: interviews.filter((row) => row.interviewType === InterviewType.MOCK).length,
      },
      recent: interviews.slice(0, 6),
      latestInsights,
      disclaimer:
        'Practice scores are coaching signals, not hiring decisions. AI-generated notes need your own judgment.',
    };
  }

  private assertCustomer(user: AuthUser) {
    if (user.role !== UserRole.CANDIDATE) {
      throw new ForbiddenException('Customer portal only');
    }
  }

  private async ensureCandidate(user: AuthUser) {
    this.assertCustomer(user);
    let candidate = await this.candidates.findOne({
      where: { organizationId: user.organizationId, email: user.email },
    });
    if (!candidate) {
      const me = await this.auth.me(user);
      candidate = await this.candidates.save(
        this.candidates.create({
          organizationId: user.organizationId,
          firstName: me.firstName,
          lastName: me.lastName,
          email: user.email,
          skills: [],
          consentGiven: true,
          consentAt: new Date(),
        }),
      );
    }
    return candidate;
  }
}
