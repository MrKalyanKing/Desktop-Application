import { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useAI } from '../../ai';
import { opsService, showNotice } from '../../hotkeys/services/ops.service';

export type CaptureMode = 'mic' | 'system' | 'both';

/** What the backend is actually recording (may differ from the preferred source). */
export interface ActualSources {
  mic: boolean;
  system: boolean;
}

const NO_SOURCES: ActualSources = { mic: false, system: false };
const CYCLE: CaptureMode[] = ['mic', 'system', 'both'];
const SOURCE_LABEL: Record<CaptureMode, string> = { mic: 'Mic', system: 'System', both: 'Mic+System' };

type CaptureStateResponse = { is_recording: boolean; mode: string };

/** Backend input-health classification, refreshed ~4x/s per source. */
export type SignalKind =
  | 'ok'
  | 'idle'
  | 'no-signal'
  | 'muted'
  | 'quiet'
  | 'noisy'
  | 'clipping'
  | 'device-error'
  | 'off';

export interface SourceHealth {
  signal: SignalKind;
  detail?: string | null;
  levelDb?: number;
  snrDb?: number | null;
}

export type VoicePhase = 'idle' | 'listening' | 'holding' | 'thinking' | 'streaming';

/** Short-lived notes shown in the listening panel. */
export type VoiceNote = { kind: 'ignored' | 'skipped' | 'echo' | 'merged'; at: number } | null;

export interface HeardQuestion {
  text: string;
  source: string;
  opId?: number;
}

const toSources = (state: CaptureStateResponse): ActualSources => {
  if (!state.is_recording) return NO_SOURCES;
  return {
    mic: state.mode === 'mic' || state.mode === 'both',
    system: state.mode === 'system' || state.mode === 'both',
  };
};

/**
 * Voice capture OFF by default (chat-first).
 * Voice audio goes DIRECTLY to Gemini (no speech-to-text).
 * Typed messages still use text Gemini via useAI.stream.
 */
export const useVoiceAgent = () => {
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [systemQuestion, setSystemQuestion] = useState('');
  const [usingScreen, setUsingScreen] = useState(false);
  const [voicePhase, setVoicePhase] = useState<VoicePhase>('idle');
  const [health, setHealth] = useState<{ mic?: SourceHealth; system?: SourceHealth }>({});
  const [speakingSource, setSpeakingSource] = useState<'mic' | 'system' | null>(null);
  const [heard, setHeard] = useState<HeardQuestion | null>(null);
  const [latency, setLatency] = useState<{ firstTokenMs: number; totalMs?: number } | null>(null);
  const [note, setNote] = useState<VoiceNote>(null);
  const [voiceError, setVoiceError] = useState<string | null>(null);
  const [captureMode, setCaptureModeState] = useState<CaptureMode>('mic');
  const [actualSources, setActualSources] = useState<ActualSources>(NO_SOURCES);
  const { presentVoiceAnswer, presentVoicePartial, cancel, setError } = useAI();
  const cancelRef = useRef(cancel);
  const presentRef = useRef(presentVoiceAnswer);
  const partialRef = useRef(presentVoicePartial);
  const setErrorRef = useRef(setError);
  const lastVoiceTextRef = useRef<string>('');
  const streamRequestRef = useRef<string>('');
  const voicePhaseRef = useRef<VoicePhase>('idle');
  const heardRef = useRef<HeardQuestion | null>(null);
  const currentVoiceOpRef = useRef<number | null>(null);
  const audioOpInFlightRef = useRef(false);

  useEffect(() => {
    cancelRef.current = cancel;
  }, [cancel]);

  useEffect(() => {
    presentRef.current = presentVoiceAnswer;
  }, [presentVoiceAnswer]);

  useEffect(() => {
    partialRef.current = presentVoicePartial;
  }, [presentVoicePartial]);

  useEffect(() => {
    setErrorRef.current = setError;
  }, [setError]);

  const isRecordingRef = useRef(false);
  const captureModeRef = useRef<CaptureMode>('mic');
  const lastTriggerRef = useRef<number>(0);
  const startedRef = useRef(false);
  const switchingRef = useRef(false);

  useEffect(() => {
    isRecordingRef.current = isRecording;
  }, [isRecording]);

  useEffect(() => {
    captureModeRef.current = captureMode;
  }, [captureMode]);

  /** Sync the UI from the backend's real capture state; never trust the requested mode. */
  const readBack = async (requested?: CaptureMode) => {
    const state = await invoke<CaptureStateResponse>('get_audio_capture_state');
    const sources = toSources(state);
    setActualSources(sources);
    setIsRecording(state.is_recording);
    isRecordingRef.current = state.is_recording;
    startedRef.current = state.is_recording;
    if (requested && state.is_recording) {
      if (requested !== 'system' && !sources.mic) {
        showNotice('error', 'Microphone unavailable — using System audio');
      } else if (requested !== 'mic' && !sources.system) {
        showNotice('error', 'System audio unavailable — using Mic');
      }
    }
    return state;
  };

  const startListening = async (mode: CaptureMode = captureModeRef.current) => {
    try {
      await invoke('start_audio_capture', { mode });
    } catch (err: any) {
      console.error('Failed to start audio capture:', err);
      setError({
        type: 'GENERAL_ERROR',
        message:
          typeof err === 'string'
            ? err
            : err?.message || 'Failed to start audio listening',
      });
    }
    try {
      await readBack(mode);
    } catch {
      // keep previous state
    }
  };

  const stopListening = async () => {
    setIsTranscribing(false);
    setVoicePhase('idle');
    setHealth({});
    setSpeakingSource(null);
    setUsingScreen(false);
    try {
      await invoke<string>('stop_audio_capture');
    } catch (err: any) {
      console.error('Failed to stop audio capture:', err);
      showNotice('error', 'Voice could not be stopped cleanly');
    }
    try {
      await readBack();
    } catch {
      setIsRecording(false);
      setActualSources(NO_SOURCES);
      startedRef.current = false;
    }
  };

  /** Shared by the mic button and F10. Ignored while a start/stop is still pending. */
  const toggleVoice = async () => {
    const now = Date.now();
    if (now - lastTriggerRef.current < 400) return;
    if (audioOpInFlightRef.current || switchingRef.current) return;
    lastTriggerRef.current = now;
    audioOpInFlightRef.current = true;

    try {
      if (!isRecordingRef.current) {
        await startListening(captureModeRef.current);
      } else {
        await stopListening();
      }
    } finally {
      audioOpInFlightRef.current = false;
    }
  };

  /**
   * Preferred source. While recording, switch the backend and read back what really
   * runs. While idle, only remember it for the next F10 (no backend call).
   */
  const changeCaptureMode = async (mode: CaptureMode) => {
    if (mode === captureModeRef.current && isRecordingRef.current) return;
    if (switchingRef.current || audioOpInFlightRef.current) return;

    if (!isRecordingRef.current) {
      setCaptureModeState(mode);
      captureModeRef.current = mode;
      return;
    }

    switchingRef.current = true;
    audioOpInFlightRef.current = true;
    setIsTranscribing(true);
    try {
      await invoke('stop_audio_capture');
      await invoke('start_audio_capture', { mode });
      setCaptureModeState(mode);
      captureModeRef.current = mode;
    } catch (err) {
      console.error('Failed to switch capture mode:', err);
      showNotice('error', `Could not switch to ${SOURCE_LABEL[mode]}`);
    } finally {
      try {
        await readBack(mode);
      } catch {
        // keep previous state
      }
      setIsTranscribing(false);
      switchingRef.current = false;
      audioOpInFlightRef.current = false;
    }
  };

  /** F11: Mic → System → Both → Mic. */
  const cycleCaptureMode = () => {
    const next = CYCLE[(CYCLE.indexOf(captureModeRef.current) + 1) % CYCLE.length];
    return changeCaptureMode(next);
  };

  useEffect(() => {
    let cancelled = false;

    const syncExistingCapture = async () => {
      try {
        const state = await invoke<CaptureStateResponse>('get_audio_capture_state');
        if (cancelled || !state.is_recording) return;

        const m =
          state.mode === 'both' || state.mode === 'system' || state.mode === 'mic'
            ? (state.mode as CaptureMode)
            : 'mic';
        setIsRecording(true);
        setActualSources(toSources(state));
        setCaptureModeState(m);
        captureModeRef.current = m;
        startedRef.current = true;
      } catch {
        // stay idle for chat
      }
    };

    syncExistingCapture();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    unlisteners.push(
      listen<{ requestId: string; opId?: number; usingScreen?: boolean }>('voice-gemini-started', (event) => {
        if (opsService.isCancelled(event.payload?.opId)) return;
        currentVoiceOpRef.current = event.payload?.opId ?? null;
        streamRequestRef.current = event.payload?.requestId || '';
        lastVoiceTextRef.current = '';
        voicePhaseRef.current = 'thinking';
        heardRef.current = null;
        setHeard(null);
        setVoiceError(null);
        setIsTranscribing(true);
        setVoicePhase('thinking');
        setTranscript('Thinking…');
        setUsingScreen(!!event.payload?.usingScreen);
      })
    );

    unlisteners.push(
      listen<{ question: string; source: string; opId?: number }>('voice-heard', (event) => {
        if (opsService.isCancelled(event.payload?.opId)) return;
        const text = event.payload?.question?.trim();
        if (!text) return;
        const h = { text, source: event.payload.source, opId: event.payload.opId };
        heardRef.current = h;
        setHeard(h);
      })
    );

    unlisteners.push(
      listen<{ source: string; reason?: string }>('audio-noise-ignored', (event) => {
        setNote({ kind: event.payload?.reason === 'echo' ? 'echo' : 'ignored', at: Date.now() });
      })
    );

    unlisteners.push(
      listen<{ merged: boolean }>('voice-utterance', (event) => {
        if (event.payload?.merged) setNote({ kind: 'merged', at: Date.now() });
      })
    );

    unlisteners.push(
      listen<{ opId?: number }>('voice-skipped', () => {
        voicePhaseRef.current = 'idle';
        setVoicePhase('idle');
        setIsTranscribing(false);
        setTranscript('');
        setNote({ kind: 'skipped', at: Date.now() });
      })
    );

    unlisteners.push(
      listen<SourceHealth & { source: 'mic' | 'system' }>('audio-health', (event) => {
        const { source, ...rest } = event.payload;
        if (source !== 'mic' && source !== 'system') return;
        setHealth((prev) => ({ ...prev, [source]: rest }));
      })
    );

    unlisteners.push(
      listen<{
        requestId: string;
        opId?: number;
        text: string;
        partial?: boolean;
        usingScreen?: boolean;
        ttftMs?: number;
      }>('voice-gemini-chunk', (event) => {
        const text = event.payload?.text?.trim();
        if (!text || opsService.isCancelled(event.payload?.opId)) return;
        if (voicePhaseRef.current !== 'streaming' && typeof event.payload?.ttftMs === 'number') {
          setLatency({ firstTokenMs: event.payload.ttftMs });
        }
        setIsTranscribing(true);
        voicePhaseRef.current = 'streaming';
        setVoicePhase('streaming');
        setTranscript('Streaming…');
        if (event.payload?.usingScreen) setUsingScreen(true);
        lastVoiceTextRef.current = text;
        partialRef.current?.(text, event.payload?.opId);
      })
    );

    unlisteners.push(
      listen<{
        answer: string;
        source: string;
        opId?: number;
        question?: string | null;
        firstTokenMs?: number;
        totalMs?: number;
      }>('voice-gemini-answer', (event) => {
        if (opsService.isCancelled(event.payload?.opId)) return;
        const answer = event.payload?.answer?.trim();
        const question = event.payload?.question?.trim() || heardRef.current?.text;
        if (typeof event.payload?.firstTokenMs === 'number') {
          setLatency({ firstTokenMs: event.payload.firstTokenMs, totalMs: event.payload.totalMs });
        }
        setIsTranscribing(false);
        voicePhaseRef.current = 'idle';
        setVoicePhase('idle');
        setTranscript('');
        currentVoiceOpRef.current = null;
        if (answer && presentRef.current) {
          lastVoiceTextRef.current = answer;
          presentRef.current(answer, event.payload.source, event.payload?.opId, {
            question,
            latencyMs: event.payload?.firstTokenMs,
          });
        }
      })
    );

    unlisteners.push(
      listen<{ message: string; source: string; opId?: number }>('voice-gemini-error', (event) => {
        if (opsService.isCancelled(event.payload?.opId)) return;
        setIsTranscribing(false);
        voicePhaseRef.current = 'idle';
        setVoicePhase('idle');
        setTranscript('');
        currentVoiceOpRef.current = null;
        const message = event.payload?.message || 'Voice request failed';
        setVoiceError(message);
        setErrorRef.current?.({ type: 'GENERAL_ERROR', message });
      })
    );

    unlisteners.push(
      listen('ai-interrupted', () => {
        if (cancelRef.current) {
          cancelRef.current();
        }
      })
    );

    unlisteners.push(
      listen<{ question: string }>('voice-system-question', (event) => {
        const q = event.payload?.question?.trim();
        if (q) setSystemQuestion(q);
      })
    );

    unlisteners.push(
      listen('audio-state-changed', (event: any) => {
        const state = event.payload?.state;
        const source = event.payload?.source === 'system' ? 'system' : 'mic';
        if (state === 'listening' || state === 'holding') {
          setSpeakingSource(source);
          voicePhaseRef.current = state;
          setIsTranscribing(true);
          setVoicePhase(state);
          setTranscript(state === 'holding' ? 'End of speech…' : 'Listening…');
        } else if (state === 'idle') {
          setSpeakingSource((s) => (s === source ? null : s));
          if (voicePhaseRef.current === 'thinking' || voicePhaseRef.current === 'streaming') {
            return;
          }
          setIsTranscribing(false);
          voicePhaseRef.current = 'idle';
          setVoicePhase('idle');
        }
      })
    );

    // F3: drop the in-progress voice answer. Recording itself keeps running.
    unlisteners.push(
      listen<{ ids: number[] }>('ks://cancelled', (event) => {
        const op = currentVoiceOpRef.current;
        if (op == null || !(event.payload?.ids ?? []).includes(op)) return;
        currentVoiceOpRef.current = null;
        voicePhaseRef.current = 'idle';
        setVoicePhase('idle');
        setIsTranscribing(false);
        setTranscript('');
      })
    );

    (window as any).__toggleVoice = toggleVoice;

    return () => {
      unlisteners.forEach((promise) => promise.then((unsub) => unsub()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const toggleVoiceDirect = () => {
    if ((window as any).__toggleVoice) {
      (window as any).__toggleVoice();
    }
  };

  return {
    isRecording,
    isTranscribing,
    transcript,
    systemQuestion,
    usingScreen,
    voicePhase,
    health,
    speakingSource,
    heard,
    latency,
    note,
    voiceError,
    captureMode,
    actualSources,
    currentVoiceOpRef,
    setCaptureMode: changeCaptureMode,
    cycleCaptureMode,
    toggleVoice: toggleVoiceDirect,
  };
};
