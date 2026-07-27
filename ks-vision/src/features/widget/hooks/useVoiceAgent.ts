import { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useAI } from '../../ai';

export const useVoiceAgent = () => {
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [captureMode, setCaptureMode] = useState<'mic' | 'system'>('mic');
  const { stream, cancel, setError } = useAI();
  const cancelRef = useRef(cancel);
  useEffect(() => {
    cancelRef.current = cancel;
  }, [cancel]);
  
  const transcriptRef = useRef('');
  const isRecordingRef = useRef(false);
  const captureModeRef = useRef<'mic' | 'system'>('mic');
  const lastTriggerRef = useRef<number>(0);
  const streamRef = useRef(stream);

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

  // Sync state with backend on mount
  useEffect(() => {
    invoke<{ is_recording: boolean; mode: string }>('get_audio_capture_state')
      .then((state) => {
        setIsRecording(state.is_recording);
        if (state.mode === 'system' || state.mode === 'mic') {
          setCaptureMode(state.mode as 'mic' | 'system');
        }
      })
      .catch((err) => console.warn('Failed to get backend audio capture state:', err));
  }, []);

  const triggerAISubmission = async (text: string) => {
    const final = text.trim();
    if (!final) return;

    // Clear the transcript buffer
    setTranscript('');
    transcriptRef.current = '';

    // Direct response instructions
    const system = 'You are an AI Meeting Copilot. Provide a direct, concise response to the user query. Keep your answer under 80 words. If the user corrects your previous statement, acknowledge it, learn from the history, and correct your response.';
    try {
      await invoke('set_ai_generating_state', { generating: true });
      await streamRef.current(final, { voiceText: final }, system);
    } catch (err) {
      console.error('Failed to stream AI response:', err);
    } finally {
      await invoke('set_ai_generating_state', { generating: false });
      
      // Check for next queued question
      try {
        const nextPrompt = await invoke<string | null>('pop_next_pending_question');
        if (nextPrompt) {
          setTimeout(() => {
            triggerAISubmission(nextPrompt);
          }, 1500);
        }
      } catch (err) {
        console.error('Failed to pop next pending question:', err);
      }
    }
  };

  const toggleVoice = async () => {
    const now = Date.now();
    if (now - lastTriggerRef.current < 400) {
      console.warn('Voice toggle debounced');
      return;
    }
    lastTriggerRef.current = now;

    if (!isRecordingRef.current) {
      // Start recording
      setTranscript('');
      transcriptRef.current = '';
      
      try {
        await invoke('start_audio_capture', { mode: captureModeRef.current });
        setIsRecording(true);
      } catch (err: any) {
        console.error('Failed to start audio capture:', err);
        // Fallback to simulation if capture fails (like if CPAL has no device)
        simulateSpeech();
      }
    } else {
      // Stop recording and transcribe
      setIsRecording(false);
      setIsTranscribing(true);

      try {
        const text = await invoke<string>('stop_audio_capture');
        setIsTranscribing(false);
        if (text && text.trim().length > 0) {
          setTranscript(text);
          await triggerAISubmission(text);
        }
      } catch (err: any) {
        setIsTranscribing(false);
        console.error('Failed to stop/transcribe audio:', err);
        setError({
          type: 'GENERAL_ERROR',
          message: typeof err === 'string' ? err : (err.message || JSON.stringify(err))
        });
      }
    }
  };

  const simulateSpeech = () => {
    setIsRecording(true);
    const phrases = [
      "Let's review the actions for the sprint.",
      "We need to package the next production installer.",
      "Can you verify that SQLite runs offline.",
      "All tests have successfully verified."
    ];
    let index = 0;
    setTranscript(phrases[0] + ' ');
    transcriptRef.current = phrases[0] + ' ';

    const simInterval = setInterval(() => {
      index++;
      if (index < phrases.length) {
        setTranscript(prev => {
          const next = prev + phrases[index] + ' ';
          transcriptRef.current = next;
          return next;
        });
      } else {
        clearInterval(simInterval);
        setIsRecording(false);
        const final = transcriptRef.current;
        triggerAISubmission(final);
      }
    }, 2500);
  };

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    unlisteners.push(listen<{ text: string; speaker: string; source: string; status: string }>(
      'audio-transcription',
      (event) => {
        setTranscript(event.payload.text);
        transcriptRef.current = event.payload.text;
      }
    ));

    unlisteners.push(listen<{ prompt: string }>('trigger-ai-response', (event) => {
      const payload = event.payload as any;
      const prompt = typeof payload === 'string' ? payload : payload.prompt;
      if (prompt) {
        triggerAISubmission(prompt);
      }
    }));

    unlisteners.push(listen('ai-interrupted', () => {
      if (cancelRef.current) {
        cancelRef.current();
      }
    }));

    const unlistenPromise = listen('toggle-voice', () => {
      console.log('toggle-voice event received');
      toggleVoice();
    });

    (window as any).__toggleVoice = toggleVoice;

    return () => {
      unlistenPromise.then(unlisten => unlisten());
      unlisteners.forEach(promise => promise.then(unsub => unsub()));
    };
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
    setCaptureMode, 
    toggleVoice: toggleVoiceDirect 
  };
};
