import React, { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Mic24Filled, Speaker224Regular, Flash24Regular } from '@fluentui/react-icons';
import type {
  ActualSources,
  SourceHealth,
  VoiceNote,
  VoicePhase,
} from '../hooks/useVoiceAgent';

interface ListeningPanelProps {
  actualSources: ActualSources;
  phase: VoicePhase;
  speakingSource: 'mic' | 'system' | null;
  health: { mic?: SourceHealth; system?: SourceHealth };
  latency: { firstTokenMs: number; totalMs?: number } | null;
  note: VoiceNote;
  voiceError: string | null;
}

type Tone = 'ready' | 'live' | 'busy' | 'warn' | 'error';

const HARD_PROBLEMS = ['device-error', 'no-signal', 'muted'];
const SOFT_PROBLEMS = ['noisy', 'clipping', 'quiet'];

const NOTE_TEXT: Record<NonNullable<VoiceNote>['kind'], string> = {
  skipped: "Didn't hear a question — ignored",
  ignored: 'Background noise ignored',
  echo: 'Ignored speaker echo — headphones help in Mic + System',
  merged: 'Joined with your previous sentence',
};

const SOURCE_LABEL = { mic: 'Mic', system: 'System' } as const;

/** -60 dBFS → 0, -15 dBFS → 1 */
const levelToFraction = (v: number) => {
  const db = 20 * Math.log10(Math.max(v, 1e-6));
  return Math.min(1, Math.max(0, (db + 60) / 45));
};

const signalLabel = (h?: SourceHealth) => {
  switch (h?.signal) {
    case 'ok':
      return 'Good';
    case 'idle':
      return 'Silent';
    case 'noisy':
      return 'Noisy';
    case 'quiet':
      return 'Too quiet';
    case 'clipping':
      return 'Too loud';
    case 'muted':
      return 'Muted';
    case 'no-signal':
      return 'No signal';
    case 'device-error':
      return 'Device error';
    default:
      return 'Starting…';
  }
};

const signalTone = (h?: SourceHealth): Tone => {
  if (!h) return 'busy';
  if (HARD_PROBLEMS.includes(h.signal)) return 'error';
  if (SOFT_PROBLEMS.includes(h.signal)) return 'warn';
  if (h.signal === 'idle') return 'busy';
  return 'ready';
};

/**
 * Voice status at a glance: is it listening, is the input healthy, what did the
 * AI hear, and how fast did it answer.
 */
export const ListeningPanel: React.FC<ListeningPanelProps> = ({
  actualSources,
  phase,
  speakingSource,
  health,
  latency,
  note,
  voiceError,
}) => {
  const meterRefs = useRef<{ mic: HTMLSpanElement | null; system: HTMLSpanElement | null }>({
    mic: null,
    system: null,
  });
  const levels = useRef({ mic: 0, system: 0 });
  const [visibleNote, setVisibleNote] = useState<VoiceNote>(null);

  useEffect(() => {
    if (!note) return;
    setVisibleNote(note);
    const t = window.setTimeout(() => setVisibleNote(null), 2600);
    return () => window.clearTimeout(t);
  }, [note]);

  // Levels arrive ~50×/s; write them straight to the DOM instead of re-rendering.
  useEffect(() => {
    let raf = 0;
    const unlisten = listen<{ volume: number; source: string }>('audio-waveform-data', (e) => {
      const src = e.payload.source === 'system' ? 'system' : 'mic';
      levels.current[src] = e.payload.volume;
    });
    const draw = () => {
      (['mic', 'system'] as const).forEach((src) => {
        const el = meterRefs.current[src];
        if (!el) return;
        const f = levelToFraction(levels.current[src]);
        el.style.transform = `scaleX(${f.toFixed(3)})`;
        levels.current[src] *= 0.85; // decay when packets stop
      });
      raf = requestAnimationFrame(draw);
    };
    raf = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(raf);
      unlisten.then((u) => u());
    };
  }, []);

  const sources = (['mic', 'system'] as const).filter((s) => actualSources[s]);
  const hardProblem = sources
    .map((s) => ({ s, h: health[s] }))
    .find(({ h }) => h && HARD_PROBLEMS.includes(h.signal));
  const softProblem = sources
    .map((s) => ({ s, h: health[s] }))
    .find(({ h }) => h && SOFT_PROBLEMS.includes(h.signal));

  let tone: Tone = 'ready';
  let status = sources.length > 1 ? 'Listening to you and the meeting' : 'Listening — ask anything';
  if (voiceError) {
    tone = 'error';
    status = voiceError;
  } else if (phase === 'listening') {
    tone = 'live';
    status = speakingSource === 'system' ? 'Hearing meeting audio…' : 'Hearing you…';
  } else if (phase === 'holding') {
    tone = 'live';
    status = 'Got it — waiting for you to finish…';
  } else if (phase === 'thinking') {
    tone = 'busy';
    status = 'Understanding the question…';
  } else if (phase === 'streaming') {
    tone = 'busy';
    status = 'Answering';
  } else if (hardProblem) {
    tone = 'error';
    status = hardProblem.h?.detail || `${SOURCE_LABEL[hardProblem.s]}: ${signalLabel(hardProblem.h)}`;
  }

  const hint =
    !voiceError && !hardProblem && softProblem?.h?.detail
      ? softProblem.h.detail
      : visibleNote
        ? NOTE_TEXT[visibleNote.kind]
        : null;

  return (
    <div className={`ks-listen ks-listen--${tone}`} role="status" aria-live="polite">
      <div className="ks-listen__status">
        <span className="ks-listen__dot" aria-hidden />
        <span className="ks-listen__text" title={status}>
          {status}
        </span>
        {latency && (
          <span className="ks-listen__latency" title="From the end of speech to the first words of the answer">
            <Flash24Regular style={{ fontSize: 11 }} />
            {(latency.firstTokenMs / 1000).toFixed(1)}s
          </span>
        )}
      </div>

      <div className="ks-listen__sources">
        {sources.map((s) => {
          const h = health[s];
          return (
            <div
              key={s}
              className={`ks-listen__source ks-listen__source--${signalTone(h)} ${speakingSource === s ? 'is-speaking' : ''}`}
              title={h?.detail || `${SOURCE_LABEL[s]}: ${signalLabel(h)}`}
            >
              {s === 'mic' ? <Mic24Filled style={{ fontSize: 12 }} /> : <Speaker224Regular style={{ fontSize: 12 }} />}
              <span className="ks-listen__source-name">{SOURCE_LABEL[s]}</span>
              <span className="ks-listen__meter" aria-hidden>
                <span
                  ref={(el) => {
                    meterRefs.current[s] = el;
                  }}
                  className="ks-listen__meter-fill"
                />
              </span>
              <span className="ks-listen__source-state">{signalLabel(h)}</span>
            </div>
          );
        })}
      </div>

      {hint && <div className="ks-listen__hint">{hint}</div>}
    </div>
  );
};
