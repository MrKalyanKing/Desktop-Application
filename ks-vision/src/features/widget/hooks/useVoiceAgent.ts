import { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useAI } from '../../ai';

export type CaptureMode = 'mic' | 'system' | 'both';

/**
 * Voice capture OFF by default (chat-first).
 * Voice audio goes DIRECTLY to Gemini (no speech-to-text).
 * Typed messages still use text Gemini via useAI.stream.
 */
export const useVoiceAgent = () => {
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [transcript, setTranscript] = useState('');
  // The question detected from system audio — updated each time new audio is heard.
  // WidgetContent reads this and pastes it into the input box immediately.
  const [systemQuestion, setSystemQuestion] = useState('');
  const [captureMode, setCaptureModeState] = useState<CaptureMode>('mic');
  const { presentVoiceAnswer, cancel, setError } = useAI();
  const cancelRef = useRef(cancel);
  const presentRef = useRef(presentVoiceAnswer);
  const setErrorRef = useRef(setError);

  useEffect(() => {
    cancelRef.current = cancel;
  }, [cancel]);

  useEffect(() => {
    presentRef.current = presentVoiceAnswer;
  }, [presentVoiceAnswer]);

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
    setIsTranscribing(false);
    try {
      await invoke<string>('stop_audio_capture');
    } catch (err: any) {
      console.error('Failed to stop audio capture:', err);
    } finally {
      startedRef.current = false;
    }
  };

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
      listen<{ answer: string; source: string }>('voice-gemini-answer', (event) => {
        const answer = event.payload?.answer;
        if (answer && presentRef.current) {
          setIsTranscribing(false);
          presentRef.current(answer, event.payload.source);
        }
      })
    );

    unlisteners.push(
      listen<{ message: string; source: string }>('voice-gemini-error', (event) => {
        setIsTranscribing(false);
        setErrorRef.current?.({
          type: 'GENERAL_ERROR',
          message: event.payload?.message || 'Voice Gemini request failed',
        });
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
        if (state === 'listening' || state === 'holding') {
          setIsTranscribing(true);
          setTranscript('Listening…');
        } else if (state === 'idle') {
          setIsTranscribing(false);
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
    // Updated whenever system audio detects a new question — paste directly into input box.
    systemQuestion,
    captureMode,
    setCaptureMode: changeCaptureMode,
    toggleVoice: toggleVoiceDirect,
  };
};
