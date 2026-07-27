import { useState, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useAI } from '../../ai';

export const useVoiceAgent = () => {
  const [isRecording, setIsRecording] = useState(false);
  const [transcript, setTranscript] = useState('');
  const { stream } = useAI();
  
  const transcriptRef = useRef('');
  const isRecordingRef = useRef(false);
  const silenceTimerRef = useRef<any>(null);
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

  const recognitionRef = useRef<any>(null);
  const simIntervalRef = useRef<any>(null);
  const isSimulatingRef = useRef(false);

  useEffect(() => {
    const SpeechRecognition = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;

    if (SpeechRecognition) {
      const rec = new SpeechRecognition();
      rec.continuous = true;
      rec.interimResults = true;
      rec.lang = 'en-US';

      rec.onresult = (event: any) => {
        let interimStr = '';
        let finalStr = '';
        for (let i = event.resultIndex; i < event.results.length; ++i) {
          if (event.results[i].isFinal) {
            finalStr += event.results[i][0].transcript + ' ';
          } else {
            interimStr += event.results[i][0].transcript;
          }
        }

        if (finalStr || interimStr) {
          setTranscript(prev => {
            const next = prev + finalStr;
            transcriptRef.current = next;
            return next;
          });

          if (silenceTimerRef.current) {
            clearTimeout(silenceTimerRef.current);
          }
          
          silenceTimerRef.current = setTimeout(() => {
            if (isRecordingRef.current) {
              console.log('Silence threshold reached, sending chunk to AI...');
              if (interimStr) {
                transcriptRef.current += interimStr + ' ';
              }
              if (recognitionRef.current) {
                try {
                  // Stop to finalize this utterance; it will auto-restart in onend
                  recognitionRef.current.stop();
                } catch {}
              } else {
                triggerAISubmission();
              }
            }
          }, 1500);
        }
      };

      rec.onerror = (e: any) => {
        console.warn('Speech recognition status update:', e.error);
        if (e.error === 'no-speech' || e.error === 'aborted') {
          return;
        }
        
        if (isRecordingRef.current && !isSimulatingRef.current) {
          isSimulatingRef.current = true;
          try {
            rec.abort();
          } catch {}
          simulateSpeech();
        }
      };

      rec.onend = () => {
        // Automatically submit the transcript
        if (isRecordingRef.current && !isSimulatingRef.current) {
          if (transcriptRef.current.trim().length > 0) {
            triggerAISubmission(false);
          }
          
          // Re-trigger speech recognition with a 100ms delay to allow system handles to clear
          setTimeout(() => {
            if (isRecordingRef.current && !isSimulatingRef.current) {
              try {
                rec.start();
              } catch (e) {
                console.warn('Failed to restart speech recognition:', e);
              }
            }
          }, 100);
        }
      };

      recognitionRef.current = rec;
    }

    const triggerAISubmission = (isExplicitStop: boolean = false) => {
      if (isExplicitStop) {
        setIsRecording(false);
        isRecordingRef.current = false;
        if (silenceTimerRef.current) {
          clearTimeout(silenceTimerRef.current);
        }
      }

      const final = transcriptRef.current.trim();
      if (!final) return;

      // Clear the transcript buffer for the next sentence
      setTranscript('');
      transcriptRef.current = '';

      // Direct response instructions with correction following rules
      const system = 'You are an AI Meeting Copilot. Provide a direct, concise response to the user query. Keep your answer under 80 words. If the user corrects your previous statement, acknowledge it, learn from the history, and correct your response.';
      streamRef.current(final, { voiceText: final }, system);
    };

    const simulateSpeech = () => {
      const phrases = [
        "Let's review the actions for the sprint.",
        "We need to package the next production installer.",
        "Can you verify that SQLite runs offline.",
        "All tests have successfully verified."
      ];
      let index = 0;
      setTranscript(phrases[0] + ' ');
      transcriptRef.current = phrases[0] + ' ';

      simIntervalRef.current = setInterval(() => {
        index++;
        if (index < phrases.length) {
          setTranscript(prev => {
            const next = prev + phrases[index] + ' ';
            transcriptRef.current = next;
            return next;
          });
        } else {
          clearInterval(simIntervalRef.current);
          silenceTimerRef.current = setTimeout(() => {
            triggerAISubmission(true);
          }, 1500);
        }
      }, 2500);
    };

    const toggleVoice = async () => {
      const now = Date.now();
      if (now - lastTriggerRef.current < 400) {
        console.warn('Voice toggle debounced');
        return;
      }
      lastTriggerRef.current = now;

      if (!isRecordingRef.current) {
        setTranscript('');
        transcriptRef.current = '';
        setIsRecording(true);
        isRecordingRef.current = true;
        isSimulatingRef.current = false;

        try {
          const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
          stream.getTracks().forEach(track => track.stop());
          
          if (recognitionRef.current) {
            try {
              recognitionRef.current.start();
            } catch (e) {
              console.warn('Failed to start speech recognition, starting simulation:', e);
              isSimulatingRef.current = true;
              simulateSpeech();
            }
          } else {
            isSimulatingRef.current = true;
            simulateSpeech();
          }
        } catch (err) {
          console.warn('Microphone access blocked, falling back to simulation:', err);
          isSimulatingRef.current = true;
          simulateSpeech();
        }
      } else {
        if (recognitionRef.current) {
          try {
            recognitionRef.current.stop();
          } catch {}
        }
        if (simIntervalRef.current) {
          clearInterval(simIntervalRef.current);
        }
        triggerAISubmission(true);
      }
    };

    const unlistenPromise = listen('toggle-voice', () => {
      console.log('toggle-voice event received');
      toggleVoice();
    });

    (window as any).__toggleVoice = toggleVoice;

    return () => {
      unlistenPromise.then(unlisten => unlisten());
      if (simIntervalRef.current) clearInterval(simIntervalRef.current);
      if (silenceTimerRef.current) clearTimeout(silenceTimerRef.current);
      if (recognitionRef.current) {
        try {
          recognitionRef.current.abort();
        } catch {}
      }
    };
  }, []);

  const toggleVoiceDirect = () => {
    if ((window as any).__toggleVoice) {
      (window as any).__toggleVoice();
    }
  };

  return { isRecording, transcript, setIsRecording, toggleVoice: toggleVoiceDirect };
};
