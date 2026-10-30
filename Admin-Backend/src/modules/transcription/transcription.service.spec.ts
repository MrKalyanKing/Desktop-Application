import { TranscriptionService } from './transcription.service';
import { SpeakerType, TranscriptSource } from '../../common/enums';

describe('TranscriptionService idempotency', () => {
  const segments = {
    findOne: jest.fn(),
    save: jest.fn(),
    create: jest.fn((row) => row),
    find: jest.fn(),
  };
  const events = { findOne: jest.fn(), save: jest.fn(), create: jest.fn((row) => row), find: jest.fn() };
  const questions = { findOne: jest.fn(), save: jest.fn(), create: jest.fn((row) => row), count: jest.fn(), find: jest.fn() };
  const responses = { findOne: jest.fn(), save: jest.fn(), create: jest.fn((row) => row), find: jest.fn() };
  const sessions = {
    get: jest.fn(),
    bumpTranscriptSequence: jest.fn(),
  };

  const service = new TranscriptionService(
    segments as any,
    events as any,
    questions as any,
    responses as any,
    sessions as any,
  );

  beforeEach(() => {
    jest.clearAllMocks();
    sessions.get.mockResolvedValue({ lastTranscriptSequence: 0, desktopSyncStatus: 'idle' });
    sessions.bumpTranscriptSequence.mockResolvedValue({});
  });

  it('skips duplicate eventIds on retry', async () => {
    segments.findOne.mockResolvedValueOnce({ id: 'existing' });
    const result = await service.ingestSegments('org', 'session', [
      {
        eventId: 'evt-1',
        speakerType: SpeakerType.INTERVIEWER,
        text: 'Tell me about a hard bug.',
        source: TranscriptSource.DESKTOP_SYSTEM,
      },
    ]);
    expect(result.duplicates).toBe(1);
    expect(result.accepted).toBe(0);
    expect(segments.save).not.toHaveBeenCalled();
  });

  it('accepts a new eventId', async () => {
    segments.findOne.mockResolvedValueOnce(null);
    questions.findOne.mockResolvedValue(null);
    questions.count.mockResolvedValue(0);
    segments.save.mockImplementation(async (row) => ({ id: 'seg-1', ...row }));
    questions.save.mockImplementation(async (row) => row);

    const result = await service.ingestSegments('org', 'session', [
      {
        eventId: 'evt-2',
        speakerType: SpeakerType.INTERVIEWER,
        text: 'How do you design retries?',
      },
    ]);
    expect(result.accepted).toBe(1);
    expect(result.duplicates).toBe(0);
  });
});
