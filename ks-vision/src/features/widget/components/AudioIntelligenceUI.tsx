import React, { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

interface AudioIntelligenceUIProps {
  isRecording: boolean;
  isTranscribing: boolean;
  captureMode: 'mic' | 'system' | 'both';
  usingScreen?: boolean;
}

/** 
 * Parakeet AI-Style Glassmorphic HUD UI
 * Sleek, modern floating visualizer with real-time intent indicators,
 * audio waveform orb, and sub-250ms streaming feedback.
 */
export const AudioIntelligenceUI: React.FC<AudioIntelligenceUIProps> = ({
  isRecording,
  isTranscribing,
  captureMode,
  usingScreen,
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
        const duration = 400;

        if (holdingTimerRef.current) clearInterval(holdingTimerRef.current);
        holdingTimerRef.current = window.setInterval(() => {
          const elapsed = Date.now() - startTime;
          const pct = Math.min(100, (elapsed / duration) * 100);
          setHoldingProgress(pct);
          if (pct >= 100 && holdingTimerRef.current) {
            clearInterval(holdingTimerRef.current);
          }
        }, 20);
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

    listen('voice-gemini-chunk', () => {
      setAudioState('answering');
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
      const amp = Math.max(3, volume * height * 0.9);
      const cycles = pitch > 0 ? Math.max(1, Math.min(12, pitch / 70)) : 4;

      const colors = [
        'rgba(34, 211, 238, 0.85)',
        'rgba(168, 85, 247, 0.75)',
        'rgba(59, 130, 246, 0.65)',
      ];

      phase += 0.18;

      for (let w = 0; w < 3; w++) {
        ctx.beginPath();
        ctx.strokeStyle = colors[w];
        ctx.lineWidth = w === 0 ? 2.5 : 1.5;

        for (let x = 0; x < width; x++) {
          const normX = x / width;
          const edgeFade = Math.sin(normX * Math.PI);
          const y =
            height / 2 +
            Math.sin(normX * Math.PI * cycles + phase + w * 1.4) * amp * edgeFade;

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
        ? 'Speech ended — thinking'
        : audioState === 'answering'
          ? usingScreen
            ? 'Streaming (with screen)'
            : 'Streaming answer'
          : 'Voice Standby';

  const badgeColor =
    audioState === 'listening'
      ? 'bg-emerald-500/20 text-emerald-400 border-emerald-500/40'
      : audioState === 'holding'
        ? 'bg-purple-500/20 text-purple-300 border-purple-500/40'
        : audioState === 'answering'
          ? 'bg-cyan-500/20 text-cyan-300 border-cyan-500/40'
          : 'bg-slate-800/40 text-slate-400 border-slate-700/40';

  return (
    <div className="flex flex-col bg-gradient-to-r from-emerald-500/10 to-cyan-500/10 border border-white/10 rounded-2xl p-2.5 w-full select-none mb-2">
      {/* Header Badge */}
      <div className="flex items-center justify-between text-[10px] font-semibold mb-2">
        <div className={`flex items-center gap-2 px-2.5 py-1 rounded-full border ${badgeColor} transition-all`}>
          <span className="w-2 h-2 rounded-full bg-current animate-pulse" />
          <span className="tracking-wide">{statusLabel}</span>
        </div>
        <span className="text-slate-400 font-mono text-[9.5px]">
          {captureMode === 'both'
            ? 'Mic + System'
            : captureMode === 'system'
              ? 'System Audio'
              : 'Mic Input'}
        </span>
      </div>

      {/* Waveform Canvas */}
      {isRecording && (
        <div className="relative h-11 bg-slate-900/60 border border-slate-800/80 rounded-xl flex items-center justify-center overflow-hidden shadow-inner">
          <canvas ref={canvasRef} width={360} height={44} className="w-full h-full" />

          {audioState === 'holding' && (
            <div className="absolute right-2.5 top-2.5 w-6 h-6 flex items-center justify-center">
              <svg className="w-6 h-6 transform -rotate-90">
                <circle
                  cx="12"
                  cy="12"
                  r="9"
                  stroke="rgba(51, 65, 85, 0.5)"
                  strokeWidth="2.5"
                  fill="transparent"
                />
                <circle
                  cx="12"
                  cy="12"
                  r="9"
                  stroke="rgb(168, 85, 247)"
                  strokeWidth="2.5"
                  fill="transparent"
                  strokeDasharray={2 * Math.PI * 9}
                  strokeDashoffset={2 * Math.PI * 9 * (1 - holdingProgress / 100)}
                />
              </svg>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
