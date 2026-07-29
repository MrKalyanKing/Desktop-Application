import React, { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

interface AudioIntelligenceUIProps {
  isRecording: boolean;
  isTranscribing: boolean;
  captureMode: 'mic' | 'system' | 'both';
}

/** Voice UI — listening / holding / answering via Gemini (no speech-to-text). */
export const AudioIntelligenceUI: React.FC<AudioIntelligenceUIProps> = ({
  isRecording,
  isTranscribing,
  captureMode,
}) => {
  const [audioState, setAudioState] = useState<'idle' | 'listening' | 'holding' | 'answering'>(
    'idle'
  );

  const volumeRef = useRef(0);
  const pitchRef = useRef(0);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const animationFrameRef = useRef<number | null>(null);

  const [holdingProgress, setHoldingProgress] = useState(0);
  const holdingTimerRef = useRef<number | null>(null);

  useEffect(() => {
    if (isTranscribing) {
      setAudioState('answering');
    } else if (isRecording) {
      setAudioState('listening');
    } else {
      setAudioState('idle');
    }
  }, [isRecording, isTranscribing]);

  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    listen<{ volume: number; pitch: number; source: string }>('audio-waveform-data', (event) => {
      volumeRef.current = event.payload.volume;
      pitchRef.current = event.payload.pitch;
    }).then((unsub) => unlisteners.push(unsub));

    listen<{ state: string }>('audio-state-changed', (event) => {
      const state = event.payload.state;
      if (state === 'listening' || state === 'holding' || state === 'idle') {
        setAudioState(state as 'idle' | 'listening' | 'holding');
      }

      if (state === 'holding') {
        setHoldingProgress(0);
        const startTime = Date.now();
        const duration = 1500;

        if (holdingTimerRef.current) clearInterval(holdingTimerRef.current);
        holdingTimerRef.current = window.setInterval(() => {
          const elapsed = Date.now() - startTime;
          const pct = Math.min(100, (elapsed / duration) * 100);
          setHoldingProgress(pct);
          if (pct >= 100 && holdingTimerRef.current) {
            clearInterval(holdingTimerRef.current);
          }
        }, 30);
      } else {
        if (holdingTimerRef.current) {
          clearInterval(holdingTimerRef.current);
          holdingTimerRef.current = null;
        }
        setHoldingProgress(0);
      }
    }).then((unsub) => unlisteners.push(unsub));

    listen('voice-gemini-answer', () => {
      setAudioState('listening');
    }).then((unsub) => unlisteners.push(unsub));

    return () => {
      unlisteners.forEach((unsub) => unsub());
      if (holdingTimerRef.current) clearInterval(holdingTimerRef.current);
    };
  }, []);

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
      const amp = Math.max(2, volume * height * 0.8);
      const cycles = pitch > 0 ? Math.max(1, Math.min(10, pitch / 80)) : 3;

      const colors = [
        'rgba(34, 211, 238, 0.45)',
        'rgba(168, 85, 247, 0.35)',
        'rgba(59, 130, 246, 0.25)',
      ];

      phase += 0.15;

      for (let w = 0; w < 3; w++) {
        ctx.beginPath();
        ctx.strokeStyle = colors[w];
        ctx.lineWidth = w === 0 ? 2 : 1.5;

        for (let x = 0; x < width; x++) {
          const normX = x / width;
          const edgeFade = Math.sin(normX * Math.PI);
          const y =
            height / 2 +
            Math.sin(normX * Math.PI * cycles + phase + w * 1.5) * amp * edgeFade;

          if (x === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
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

  const statusLabel =
    audioState === 'listening'
      ? 'Listening'
      : audioState === 'holding'
        ? 'End of speech — sending to Gemini'
        : audioState === 'answering'
          ? 'Gemini answering'
          : 'Voice standby';

  return (
    <div className="flex flex-col bg-slate-900/40 border border-slate-800/40 rounded-lg p-2.5 w-full select-none mb-1.5 transition-all">
      <div className="flex items-center justify-between text-[9px] font-bold text-cyan-400 mb-1.5 uppercase tracking-wide">
        <div className="flex items-center gap-1.5">
          <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-ping" />
          <span>{statusLabel}</span>
        </div>
        <span className="text-slate-500 font-mono">
          Source:{' '}
          {captureMode === 'both'
            ? 'Mic + System'
            : captureMode === 'system'
              ? 'System Loopback'
              : 'Mic Input'}
        </span>
      </div>

      {isRecording && (
        <div className="relative h-9 bg-slate-950/40 border border-slate-900/60 rounded flex items-center justify-center overflow-hidden">
          <canvas ref={canvasRef} width={320} height={36} className="w-full h-full" />

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
    </div>
  );
};
