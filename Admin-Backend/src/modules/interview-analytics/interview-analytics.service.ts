import { Injectable } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import {
  AnalyticsKind,
  AnalyticsOrigin,
  HumanReviewStatus,
  SpeakerType,
} from '../../common/enums';
import { InterviewSessionsService } from '../interview-sessions/interview-sessions.service';
import { TranscriptSegment } from '../transcription/transcript-segment.entity';
import { InterviewAnalytics } from './interview-analytics.entity';

const FILLERS = ['um', 'uh', 'like', 'you know', 'so', 'actually', 'basically'];

@Injectable()
export class InterviewAnalyticsService {
  constructor(
    @InjectRepository(InterviewAnalytics)
    private readonly analytics: Repository<InterviewAnalytics>,
    @InjectRepository(TranscriptSegment)
    private readonly segments: Repository<TranscriptSegment>,
    private readonly sessions: InterviewSessionsService,
  ) {}

  list(sessionId: string) {
    return this.analytics.find({ where: { sessionId }, order: { generatedAt: 'DESC' } });
  }

  async generate(organizationId: string, sessionId: string) {
    await this.sessions.get(organizationId, sessionId);
    const segments = await this.segments.find({
      where: { sessionId },
      order: { sequence: 'ASC' },
    });

    const measured = this.computeMeasured(segments);
    const ai = this.interpret(segments, measured);

    const rows = await this.analytics.save([
      this.analytics.create({
        sessionId,
        kind: AnalyticsKind.SESSION_SUMMARY,
        origin: AnalyticsOrigin.MEASURED,
        provider: 'parakeet-measured',
        isAiGenerated: false,
        humanReviewStatus: HumanReviewStatus.ACCEPTED,
        payload: measured,
      }),
      this.analytics.create({
        sessionId,
        kind: AnalyticsKind.CANDIDATE_RESPONSE,
        origin: AnalyticsOrigin.AI_GENERATED,
        provider: 'parakeet-heuristic',
        model: 'heuristic-v1',
        isAiGenerated: true,
        humanReviewStatus: HumanReviewStatus.PENDING,
        payload: {
          disclaimer:
            'AI-generated assessment for review. This is not an objective measure of candidate ability. Do not infer honesty, personality, mental state, or emotion.',
          insights: ai.candidate,
        },
      }),
      this.analytics.create({
        sessionId,
        kind: AnalyticsKind.INTERVIEWER,
        origin: AnalyticsOrigin.AI_GENERATED,
        provider: 'parakeet-heuristic',
        model: 'heuristic-v1',
        isAiGenerated: true,
        humanReviewStatus: HumanReviewStatus.PENDING,
        payload: {
          disclaimer:
            'AI-generated interpretation of interviewer patterns. Measured speaking times are separate from these labels.',
          insights: ai.interviewer,
        },
      }),
    ]);

    return rows;
  }

  async review(organizationId: string, id: string, status: HumanReviewStatus, notes?: string) {
    const row = await this.analytics.findOne({ where: { id } });
    if (!row) return null;
    await this.sessions.get(organizationId, row.sessionId);
    row.humanReviewStatus = status;
    row.humanReviewNotes = notes ?? null;
    return this.analytics.save(row);
  }

  private computeMeasured(segments: TranscriptSegment[]) {
    const bySpeaker = (type: SpeakerType) => segments.filter((s) => s.speakerType === type);
    const duration = (items: TranscriptSegment[]) =>
      items.reduce((sum, item) => sum + Math.max(0, item.endMs - item.startMs), 0);
    const words = (items: TranscriptSegment[]) =>
      items.reduce((sum, item) => sum + item.text.trim().split(/\s+/).filter(Boolean).length, 0);
    const fillers = (items: TranscriptSegment[]) => {
      const blob = items.map((i) => i.text.toLowerCase()).join(' ');
      return FILLERS.reduce(
        (acc, word) => {
          const re = new RegExp(`\\b${word.replace(' ', '\\s+')}\\b`, 'g');
          acc[word] = blob.match(re)?.length ?? 0;
          return acc;
        },
        {} as Record<string, number>,
      );
    };

    const candidate = bySpeaker(SpeakerType.CANDIDATE);
    const interviewer = bySpeaker(SpeakerType.INTERVIEWER);
    const copilot = bySpeaker(SpeakerType.COPILOT);
    const candidateMs = duration(candidate);
    const interviewerMs = duration(interviewer);

    return {
      origin: AnalyticsOrigin.MEASURED,
      segmentCount: segments.length,
      candidate: {
        segments: candidate.length,
        speakingMs: candidateMs,
        wordCount: words(candidate),
        fillerCounts: fillers(candidate),
      },
      interviewer: {
        segments: interviewer.length,
        speakingMs: interviewerMs,
        wordCount: words(interviewer),
        questionCount: interviewer.length,
      },
      copilot: { segments: copilot.length, wordCount: words(copilot) },
      speakingRatio:
        candidateMs + interviewerMs === 0
          ? null
          : {
              candidate: candidateMs / (candidateMs + interviewerMs),
              interviewer: interviewerMs / (candidateMs + interviewerMs),
            },
      mapping: segments.map((segment) => ({
        eventId: segment.eventId,
        sequence: segment.sequence,
        speakerType: segment.speakerType,
        startMs: segment.startMs,
        endMs: segment.endMs,
      })),
    };
  }

  private interpret(segments: TranscriptSegment[], measured: ReturnType<InterviewAnalyticsService['computeMeasured']>) {
    const candidateTexts = segments
      .filter((s) => s.speakerType === SpeakerType.CANDIDATE)
      .map((s) => s.text);
    const interviewerTexts = segments
      .filter((s) => s.speakerType === SpeakerType.INTERVIEWER)
      .map((s) => s.text);

    const avgLen =
      candidateTexts.reduce((n, t) => n + t.split(/\s+/).length, 0) /
      Math.max(1, candidateTexts.length);
    const usesExamples = candidateTexts.some((t) =>
      /for example|for instance|one time|we built|i implemented/i.test(t),
    );
    const starHints = candidateTexts.filter((t) =>
      /situation|task|action|result|i led|we decided/i.test(t),
    ).length;

    return {
      candidate: [
        {
          dimension: 'completeness',
          label: avgLen > 40 ? 'Generally developed answers' : 'Often brief answers',
          evidenceSequences: segments
            .filter((s) => s.speakerType === SpeakerType.CANDIDATE)
            .slice(0, 3)
            .map((s) => s.sequence),
        },
        {
          dimension: 'examples',
          label: usesExamples
            ? 'Uses concrete examples in at least one answer'
            : 'Limited explicit examples detected in transcript',
          evidenceSequences: [],
        },
        {
          dimension: 'star_structure',
          label:
            starHints > 0
              ? 'Some answers resemble STAR-style structure'
              : 'STAR-style structure not clearly present',
          evidenceSequences: [],
        },
        {
          dimension: 'verbosity',
          label: avgLen > 90 ? 'Answers tend to be long' : 'Answers tend to be concise',
          evidenceSequences: [],
        },
      ],
      interviewer: [
        {
          dimension: 'question_volume',
          label: `${measured.interviewer.questionCount} interviewer utterances captured`,
        },
        {
          dimension: 'follow_ups',
          label:
            interviewerTexts.filter((t) => /can you|tell me more|why|how did/i.test(t)).length > 1
              ? 'Follow-up phrasing appears more than once'
              : 'Limited follow-up phrasing detected',
        },
        {
          dimension: 'speaking_balance',
          label: measured.speakingRatio
            ? `Candidate share of timed speech: ${Math.round(measured.speakingRatio.candidate * 100)}%`
            : 'Speaking ratio unavailable because timestamps were not provided',
        },
      ],
    };
  }
}
