import { Injectable } from '@nestjs/common';
import { InjectRepository } from '@nestjs/typeorm';
import { Repository } from 'typeorm';
import {
  InterviewEventType,
  QuestionAskedBy,
  SpeakerType,
  TranscriptSource,
} from '../../common/enums';
import { InterviewSessionsService } from '../interview-sessions/interview-sessions.service';
import { InterviewQuestion } from '../interview-questions/interview-question.entity';
import { InterviewResponse } from '../interview-responses/interview-response.entity';
import { InterviewEvent } from './interview-event.entity';
import { TranscriptSegment } from './transcript-segment.entity';

export interface TranscriptSegmentInput {
  eventId: string;
  speakerType: SpeakerType;
  text: string;
  startMs?: number;
  endMs?: number;
  sequence?: number;
  source?: TranscriptSource;
  confidence?: number;
  metadata?: Record<string, unknown>;
  occurredAt?: string;
  questionId?: string;
}

export interface SessionEventInput {
  eventId: string;
  eventType: InterviewEventType;
  payload?: Record<string, unknown>;
  occurredAt?: string;
}

@Injectable()
export class TranscriptionService {
  constructor(
    @InjectRepository(TranscriptSegment)
    private readonly segments: Repository<TranscriptSegment>,
    @InjectRepository(InterviewEvent)
    private readonly events: Repository<InterviewEvent>,
    @InjectRepository(InterviewQuestion)
    private readonly questions: Repository<InterviewQuestion>,
    @InjectRepository(InterviewResponse)
    private readonly responses: Repository<InterviewResponse>,
    private readonly sessions: InterviewSessionsService,
  ) {}

  assertSession(organizationId: string, sessionId: string) {
    return this.sessions.get(organizationId, sessionId);
  }

  listSegments(sessionId: string) {
    return this.segments.find({ where: { sessionId }, order: { sequence: 'ASC' } });
  }

  listEvents(sessionId: string) {
    return this.events.find({ where: { sessionId }, order: { occurredAt: 'ASC' } });
  }

  listQuestions(sessionId: string) {
    return this.questions.find({ where: { sessionId }, order: { sequence: 'ASC' } });
  }

  listResponses(sessionId: string) {
    return this.responses.find({ where: { sessionId }, order: { createdAt: 'ASC' } });
  }

  async ingestSegments(
    organizationId: string,
    sessionId: string,
    items: TranscriptSegmentInput[],
  ) {
    const session = await this.sessions.get(organizationId, sessionId);
    const accepted: TranscriptSegment[] = [];
    const duplicates: string[] = [];

    for (const item of items) {
      const existing = await this.segments.findOne({
        where: { sessionId, eventId: item.eventId },
      });
      if (existing) {
        duplicates.push(item.eventId);
        continue;
      }
      const sequence = item.sequence ?? session.lastTranscriptSequence + 1;
      const saved = await this.segments.save(
        this.segments.create({
          sessionId,
          eventId: item.eventId,
          speakerType: item.speakerType,
          text: item.text,
          startMs: item.startMs ?? 0,
          endMs: item.endMs ?? 0,
          sequence,
          source: item.source ?? TranscriptSource.DESKTOP_GEMINI,
          confidence: item.confidence ?? null,
          metadata: item.metadata ?? null,
        }),
      );
      accepted.push(saved);
      session.lastTranscriptSequence = Math.max(session.lastTranscriptSequence, sequence);

      if (item.speakerType === SpeakerType.INTERVIEWER) {
        await this.upsertQuestion(sessionId, item);
      }
      if (item.speakerType === SpeakerType.CANDIDATE) {
        await this.upsertResponse(sessionId, item);
      }
    }

    await this.sessions.bumpTranscriptSequence(
      organizationId,
      sessionId,
      session.lastTranscriptSequence,
    );

    return {
      accepted: accepted.length,
      duplicates: duplicates.length,
      lastSequence: session.lastTranscriptSequence,
      segments: accepted,
    };
  }

  async ingestEvents(organizationId: string, sessionId: string, items: SessionEventInput[]) {
    await this.sessions.get(organizationId, sessionId);
    const accepted: InterviewEvent[] = [];
    let duplicates = 0;

    for (const item of items) {
      const existing = await this.events.findOne({
        where: { sessionId, eventId: item.eventId },
      });
      if (existing) {
        duplicates += 1;
        continue;
      }
      accepted.push(
        await this.events.save(
          this.events.create({
            sessionId,
            eventId: item.eventId,
            eventType: item.eventType,
            payload: item.payload ?? {},
            occurredAt: item.occurredAt ? new Date(item.occurredAt) : new Date(),
          }),
        ),
      );
    }

    return { accepted: accepted.length, duplicates, events: accepted };
  }

  async addQuestion(
    organizationId: string,
    sessionId: string,
    data: { text: string; askedBy?: QuestionAskedBy; category?: string; eventId?: string },
  ) {
    await this.sessions.get(organizationId, sessionId);
    const count = await this.questions.count({ where: { sessionId } });
    return this.questions.save(
      this.questions.create({
        sessionId,
        text: data.text,
        askedBy: data.askedBy ?? QuestionAskedBy.INTERVIEWER,
        category: data.category ?? null,
        sequence: count + 1,
        askedAt: new Date(),
        eventId: data.eventId ?? null,
      }),
    );
  }

  private async upsertQuestion(sessionId: string, item: TranscriptSegmentInput) {
    if (item.eventId) {
      const existing = await this.questions.findOne({ where: { sessionId, eventId: item.eventId } });
      if (existing) return existing;
    }
    const count = await this.questions.count({ where: { sessionId } });
    return this.questions.save(
      this.questions.create({
        sessionId,
        text: item.text,
        askedBy: QuestionAskedBy.INTERVIEWER,
        sequence: count + 1,
        askedAt: new Date(),
        eventId: item.eventId,
      }),
    );
  }

  private async upsertResponse(sessionId: string, item: TranscriptSegmentInput) {
    if (item.eventId) {
      const existing = await this.responses.findOne({
        where: { sessionId, eventId: item.eventId },
      });
      if (existing) return existing;
    }
    return this.responses.save(
      this.responses.create({
        sessionId,
        speaker: SpeakerType.CANDIDATE,
        text: item.text,
        startMs: item.startMs ?? null,
        endMs: item.endMs ?? null,
        durationMs:
          item.startMs != null && item.endMs != null ? item.endMs - item.startMs : null,
        eventId: item.eventId,
        questionId: item.questionId ?? null,
      }),
    );
  }
}
