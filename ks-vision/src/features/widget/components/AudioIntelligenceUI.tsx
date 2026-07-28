import React, { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

interface AudioIntelligenceUIProps {
  isRecording: boolean;
  isTranscribing: boolean;
  captureMode: 'mic' | 'system' | 'both';
}

interface Question {
  text: string;
  is_question: boolean;
  dependencies: number[];
}

export const AudioIntelligenceUI: React.FC<AudioIntelligenceUIProps> = ({
  isRecording,
  isTranscribing,
  captureMode,
}) => {
  const [audioState, setAudioState] = useState<'idle' | 'listening' | 'holding' | 'transcribing' | 'parsing_questions' | 'awaiting_clarification'>('idle');
  const [transcript, setTranscript] = useState('');
  const [isRevising, setIsRevising] = useState(false);
  const [questions, setQuestions] = useState<Question[]>([]);
  const [activeQuestionIndex, setActiveQuestionIndex] = useState(0);
  const [clarification, setClarification] = useState<{ text: string; speaker: string } | null>(null);
  
  // Real-time volume and pitch for canvas animation
  const volumeRef = useRef(0);
  const pitchRef = useRef(0);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const animationFrameRef = useRef<number | null>(null);
  
  // Holding state circular progress
  const [holdingProgress, setHoldingProgress] = useState(0);
  const holdingTimerRef = useRef<number | null>(null);

  // Sync state with props
  useEffect(() => {
    if (isTranscribing) {
      setAudioState('transcribing');
    } else if (isRecording) {
      setAudioState('listening');
    } else {
      setAudioState('idle');
      setTranscript('');
      setQuestions([]);
    }
  }, [isRecording, isTranscribing]);

  // Subscribe to tauri backend events
  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    // Waveform data listener
    listen<{ volume: number; pitch: number; source: string }>('audio-waveform-data', (event) => {
      volumeRef.current = event.payload.volume;
      pitchRef.current = event.payload.pitch;
    }).then(unsub => unlisteners.push(unsub));

    // Audio VAD state changed listener
    listen<{ state: string }>('audio-state-changed', (event) => {
      const state = event.payload.state as any;
      setAudioState(state);

      if (state === 'holding') {
        // Start progress ring animation (VAD timeout is 1.5s - 2.5s. We default progress fill over 1.5s)
        setHoldingProgress(0);
        const startTime = Date.now();
        const duration = 1500;

        if (holdingTimerRef.current) clearInterval(holdingTimerRef.current);
        holdingTimerRef.current = window.setInterval(() => {
          const elapsed = Date.now() - startTime;
          const pct = Math.min(100, (elapsed / duration) * 100);
          setHoldingProgress(pct);
          if (pct >= 100) {
            if (holdingTimerRef.current) clearInterval(holdingTimerRef.current);
          }
        }, 30);
      } else {
        if (holdingTimerRef.current) {
          clearInterval(holdingTimerRef.current);
          holdingTimerRef.current = null;
        }
        setHoldingProgress(0);
      }
    }).then(unsub => unlisteners.push(unsub));

    // Audio transcription chunk listener
    listen<{ text: string; speaker: string; source: string; status: string }>('audio-transcription', (event) => {
      const text = event.payload.text;
      
      // Detect restarts ("I mean", "actually") to flash "revising..."
      const lower = text.toLowerCase();
      if (lower.includes('i mean') || lower.includes('actually') || lower.includes('wait') || lower.includes('sorry')) {
        setIsRevising(true);
        setTimeout(() => setIsRevising(false), 800);
      }
      
      setTranscript(text);
      setAudioState('idle');
    }).then(unsub => unlisteners.push(unsub));

    // Audio clarification needed listener
    listen<{ text: string; speaker: string; source: string }>('audio-clarification-needed', (event) => {
      setClarification({ text: event.payload.text, speaker: event.payload.speaker });
      setAudioState('awaiting_clarification');
    }).then(unsub => unlisteners.push(unsub));

    // Questions parsed listener
    listen<{ questions: Question[] }>('questions-parsed', (event) => {
      setQuestions(event.payload.questions);
      setActiveQuestionIndex(0);
      if (event.payload.questions.length > 0) {
        setAudioState('parsing_questions');
      }
    }).then(unsub => unlisteners.push(unsub));

    // Processing question index listener
    listen<{ text: string; pending_count: number }>('processing-question', (event) => {
      setAudioState('parsing_questions');
    }).then(unsub => unlisteners.push(unsub));

    return () => {
      unlisteners.forEach(unsub => unsub());
      if (holdingTimerRef.current) clearInterval(holdingTimerRef.current);
    };
  }, []);

  // Webform Canvas Animation
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    let phase = 0;

    const render = () => {
      const width = canvas.width;
      const height = canvas.height;
      ctx.clearRect(0, 0, width, height);

      const volume = volumeRef.current;
      const pitch = pitchRef.current;

      // Base voice properties
      const amp = Math.max(2, volume * height * 0.8);
      // Map pitch to number of wave cycles
      const cycles = pitch > 0 ? Math.max(1, Math.min(10, pitch / 80)) : 3;

      // Draw three overlaid translucent sine waves (gemini-style voice vibe)
      const colors = [
        'rgba(34, 211, 238, 0.45)', // Cyan
        'rgba(168, 85, 247, 0.35)', // Purple
        'rgba(59, 130, 246, 0.25)', // Blue
      ];

      phase += 0.15;

      for (let w = 0; w < 3; w++) {
        ctx.beginPath();
        ctx.strokeStyle = colors[w];
        ctx.lineWidth = w === 0 ? 2 : 1.5;

        for (let x = 0; x < width; x++) {
          // Normalize position
          const normX = x / width;
          // Apply windowing function so the wave fades out at the edges
          const edgeFade = Math.sin(normX * Math.PI);
          // Sine wave equation
          const y = (height / 2) + Math.sin(normX * Math.PI * cycles + phase + (w * 1.5)) * amp * edgeFade;

          if (x === 0) {
            ctx.moveTo(x, y);
          } else {
            ctx.lineTo(x, y);
          }
        }
        ctx.stroke();
      }

      animationFrameRef.current = requestAnimationFrame(render);
    };

    render();

    return () => {
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [audioState]);

  const handleConfirmClarification = async (confirmedText: string) => {
    setClarification(null);
    setAudioState('idle');
    // Save confirmed text to history
    await invoke('db_save_message_cmd', {
      role: 'user',
      content: confirmedText,
      source: 'Voice',
    });
  };

  const handleDiscardClarification = () => {
    setClarification(null);
    setAudioState('idle');
  };

  return (
    <div className="flex flex-col bg-slate-900/40 border border-slate-800/40 rounded-lg p-2.5 w-full select-none mb-1.5 transition-all">
      {/* Top Header info */}
      <div className="flex items-center justify-between text-[9px] font-bold text-cyan-400 mb-1.5 uppercase tracking-wide">
        <div className="flex items-center gap-1.5">
          <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-ping" />
          <span>
            {audioState === 'listening' && 'Listening to speech'}
            {audioState === 'holding' && 'Thinking (breath/pause)'}
            {audioState === 'transcribing' && 'Transcribing speech'}
            {audioState === 'parsing_questions' && 'Routing questions'}
            {audioState === 'awaiting_clarification' && 'Ambiguous speech'}
            {audioState === 'idle' && 'Voice Engine Standby'}
          </span>
        </div>
        <span className="text-slate-500 font-mono">
          Source:{' '}
          {captureMode === 'both'
            ? 'Mic + System (always on)'
            : captureMode === 'system'
              ? 'System Loopback'
              : 'Mic Input'}
        </span>
      </div>

      {/* Visual Waveform Canvas */}
      {isRecording && (
        <div className="relative h-9 bg-slate-950/40 border border-slate-900/60 rounded flex items-center justify-center overflow-hidden mb-1.5">
          <canvas
            ref={canvasRef}
            width={320}
            height={36}
            className="w-full h-full"
          />
          
          {/* Circular holding progress ring */}
          {audioState === 'holding' && (
            <div className="absolute right-2 top-2 w-5 h-5 flex items-center justify-center">
              <svg className="w-5 h-5 transform -rotate-90">
                <circle
                  cx="10"
                  cy="10"
                  r="8"
                  stroke="rgba(30, 41, 59, 0.5)"
                  strokeWidth="2"
                  fill="transparent"
                />
                <circle
                  cx="10"
                  cy="10"
                  r="8"
                  stroke="rgb(34, 211, 238)"
                  strokeWidth="2"
                  fill="transparent"
                  strokeDasharray={2 * Math.PI * 8}
                  strokeDashoffset={2 * Math.PI * 8 * (1 - holdingProgress / 100)}
                />
              </svg>
            </div>
          )}
        </div>
      )}

      {/* Sentence Restart Revision Indicator */}
      {isRevising && (
        <div className="text-[9px] font-bold text-amber-400 bg-amber-950/20 px-2 py-0.5 rounded border border-amber-900/30 text-center animate-pulse mb-1.5 select-none">
          ✍️ REVISING (Self-correction detected, removing false start)...
        </div>
      )}

      {/* Live Transcript Display */}
      {transcript && (
        <div className={`text-[10px] bg-slate-950/60 p-2 rounded border border-slate-800/40 font-mono text-slate-200 leading-normal mb-1.5 italic ${isRevising ? 'opacity-50' : ''}`}>
          "{transcript}"
        </div>
      )}

      {/* Multi-Question Cards Queue */}
      {questions.length > 0 && (
        <div className="space-y-1.5 mb-1.5">
          <div className="text-[8.5px] font-bold text-purple-400 flex justify-between items-center px-1">
            <span>🧩 DETECTED QUESTIONS QUEUE</span>
            <span>Completed 0 of {questions.length}</span>
          </div>
          <div className="grid grid-cols-1 gap-1">
            {questions.map((q, idx) => (
              <div
                key={idx}
                className={`p-2 rounded border text-[9px] flex gap-2 items-center ${
                  idx === activeQuestionIndex
                    ? 'bg-purple-950/20 border-purple-800/60 text-purple-200 font-bold'
                    : 'bg-slate-950/20 border-slate-850/20 text-slate-400'
                }`}
              >
                <span className={`w-3.5 h-3.5 rounded-full flex items-center justify-center text-[8px] ${
                  idx === activeQuestionIndex
                    ? 'bg-purple-500 text-slate-950 font-black'
                    : 'bg-slate-800 text-slate-400'
                }`}>
                  Q{idx + 1}
                </span>
                <span className="flex-1 truncate">{q.text}</span>
                {q.dependencies.length > 0 && (
                  <span className="text-[7.5px] font-mono text-amber-500 border border-amber-500/20 px-1 rounded">
                    Needs Q{q.dependencies.join(', ')}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Clarification Overlay Dialog */}
      {audioState === 'awaiting_clarification' && clarification && (
        <div className="bg-rose-950/20 border border-rose-900/40 rounded p-2 text-slate-200 flex flex-col gap-2">
          <div className="flex items-center gap-1.5 text-rose-400 font-bold text-[9px]">
            <span>⚠️ UNCLEAR INPUT RECEIVED</span>
          </div>
          <p className="text-[9px] leading-relaxed font-medium italic">
            I didn't catch that clearly. Sounds like: "{clarification.text}"
          </p>
          <div className="flex gap-1.5">
            <button
              onClick={() => handleConfirmClarification(clarification.text)}
              className="flex-1 h-5 bg-rose-900/40 border border-rose-800/60 hover:bg-rose-900/60 text-[8px] font-bold text-rose-200 rounded cursor-pointer transition-colors"
            >
              Confirm ("Did you mean...?")
            </button>
            <button
              onClick={handleDiscardClarification}
              className="px-2 h-5 bg-slate-950 border border-slate-800 text-[8px] hover:bg-slate-900 font-bold text-slate-400 rounded cursor-pointer transition-colors"
            >
              Discard
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
