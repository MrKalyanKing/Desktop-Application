import { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useAI } from '../../ai';

export type CaptureMode = 'mic' | 'system' | 'both';

/**
 * Voice capture is OFF by default so chat is ready for typed messages.
 * User starts listening with the mic button and picks mic / system / both as needed.
 *
 * Mic button pauses/resumes; mode toggle switches source while listening.
 */
export const useVoiceAgent = () => {
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [captureMode, setCaptureModeState] = useState<CaptureMode>('mic');
  const { stream, cancel, setError } = useAI();
  const cancelRef = useRef(cancel);
  useEffect(() => {
    cancelRef.current = cancel;
  }, [cancel]);

  const transcriptRef = useRef('');
  const isRecordingRef = useRef(false);
  const captureModeRef = useRef<CaptureMode>('mic');
  const lastTriggerRef = useRef<number>(0);
  const streamRef = useRef(stream);
  const startedRef = useRef(false);
  const switchingRef = useRef(false);

  useEffect(() => {
    streamRef.current = stream;
  }, [stream]);

  useEffect(() => {
    transcriptRef.current = transcript;
  }, [transcript]);

  useEffect(() => {
    isRecordingRef.current = isRecording;
  }, [isRecording]);

  useEffect(() => {
    captureModeRef.current = captureMode;
  }, [captureMode]);

  const triggerAISubmission = async (text: string) => {
    const final = text.trim();
    if (!final) return;

    setTranscript('');
    transcriptRef.current = '';

    const system =
      'You are an AI Meeting Copilot. Provide a direct, concise response to the user query. Keep your answer under 80 words. If the user corrects your previous statement, acknowledge it, learn from the history, and correct your response.';
    try {
      await invoke('set_ai_generating_state', { generating: true });
      await streamRef.current(final, { voiceText: final }, system);
    } catch (err) {
      console.error('Failed to stream AI response:', err);
    } finally {
      await invoke('set_ai_generating_state', { generating: false });

      try {
        const nextPrompt = await invoke<string | null>('pop_next_pending_question');
        if (nextPrompt) {
          setTimeout(() => {
            triggerAISubmission(nextPrompt);
          }, 200);
        }
      } catch (err) {
        console.error('Failed to pop next pending question:', err);
      }
    }
  };

  const startListening = async (mode: CaptureMode = captureModeRef.current) => {
    try {
      await invoke('start_audio_capture', { mode });
      setCaptureModeState(mode);
      captureModeRef.current = mode;
      setIsRecording(true);
      startedRef.current = true;
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
  };

  const stopListening = async () => {
    setIsRecording(false);
    setIsTranscribing(true);
    try {
      await invoke<string>('stop_audio_capture');
    } catch (err: any) {
      console.error('Failed to stop audio capture:', err);
    } finally {
      setIsTranscribing(false);
      startedRef.current = false;
    }
  };

  /** Mic button = pause / resume current mode (mic, system, or both). */
  const toggleVoice = async () => {
    const now = Date.now();
    if (now - lastTriggerRef.current < 400) return;
    lastTriggerRef.current = now;

    if (!isRecordingRef.current) {
      await startListening(captureModeRef.current);
    } else {
      await stopListening();
    }
  };

  /** Switch Mic / System / Both — keeps listening if already active. */
  const changeCaptureMode = async (mode: CaptureMode) => {
    if (mode === captureModeRef.current && isRecordingRef.current) return;
    if (switchingRef.current) return;
    switchingRef.current = true;

    setCaptureModeState(mode);
    captureModeRef.current = mode;

    try {
      if (isRecordingRef.current) {
        setIsTranscribing(true);
        await invoke('stop_audio_capture');
        await invoke('start_audio_capture', { mode });
        setIsRecording(true);
        startedRef.current = true;
      }
      // If paused, only store preference — resume will use captureModeRef
    } catch (err) {
      console.error('Failed to switch capture mode:', err);
      setError({
        type: 'GENERAL_ERROR',
        message: 'Failed to switch audio source',
      });
    } finally {
      setIsTranscribing(false);
      switchingRef.current = false;
    }
  };

  // Sync UI if capture was already running (e.g. hot reload) — never auto-start.
  useEffect(() => {
    let cancelled = false;

    const syncExistingCapture = async () => {
      try {
        const state = await invoke<{ is_recording: boolean; mode: string }>(
          'get_audio_capture_state'
        );
        if (cancelled || !state.is_recording) return;

        const m =
          state.mode === 'both' || state.mode === 'system' || state.mode === 'mic'
            ? (state.mode as CaptureMode)
            : 'mic';
        setIsRecording(true);
        setCaptureModeState(m);
        captureModeRef.current = m;
        startedRef.current = true;
      } catch {
        // ignore — stay idle for chat
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
      listen<{ text: string; speaker: string; source: string; status: string }>(
        'audio-transcription',
        (event) => {
          setTranscript(event.payload.text);
          transcriptRef.current = event.payload.text;
          setIsRecording(true);
          // Partial = still speaking / decoding; final = endpoint complete
          setIsTranscribing(event.payload.status === 'partial');
        }
      )
    );

    unlisteners.push(
      listen<{ prompt: string }>('trigger-ai-response', (event) => {
        const payload = event.payload as any;
        const prompt = typeof payload === 'string' ? payload : payload.prompt;
        if (prompt) {
          triggerAISubmission(prompt);
        }
      })
    );

    unlisteners.push(
      listen('ai-interrupted', () => {
        if (cancelRef.current) {
          cancelRef.current();
        }
      })
    );

    const unlistenPromise = listen('toggle-voice', () => {
      toggleVoice();
    });

    (window as any).__toggleVoice = toggleVoice;

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
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
    captureMode,
    setCaptureMode: changeCaptureMode,
    toggleVoice: toggleVoiceDirect,
  };
};
