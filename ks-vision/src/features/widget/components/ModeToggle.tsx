import React from 'react';
import type { CaptureMode } from '../hooks/useVoiceAgent';

interface ModeToggleProps {
  mode: CaptureMode;
  onChange: (mode: CaptureMode) => void;
  disabled?: boolean;
}

export const ModeToggle: React.FC<ModeToggleProps> = ({ mode, onChange, disabled }) => {
  const btn = (id: CaptureMode, label: string, title: string) => (
    <button
      type="button"
      onClick={() => onChange(id)}
      disabled={disabled}
      className={`flex-1 flex items-center justify-center gap-1 py-1 px-1.5 rounded-sm transition-all cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed ${
        mode === id
          ? 'bg-cyan-950/80 text-cyan-300 border border-cyan-800/30'
          : 'text-slate-400 hover:text-slate-200'
      }`}
      title={title}
    >
      {label}
    </button>
  );

  return (
    <div className="flex flex-col gap-0.5 w-full">
      <div className="flex bg-slate-950/40 border border-slate-800/40 rounded-md p-0.5 select-none text-[9px] font-bold w-full">
        {btn('both', '⚡ Both', 'Listen to your mic AND system audio together')}
        {btn('mic', '🎤 My Voice', 'Listen to your microphone only')}
        {btn('system', '🔊 System', 'Listen to Meet / Teams / YouTube only')}
      </div>
      <div className="text-[8px] text-slate-500 px-0.5 truncate">
        {mode === 'both' && 'Selected: mic + system'}
        {mode === 'mic' && 'Selected: your microphone'}
        {mode === 'system' && 'Selected: system audio'}
      </div>
    </div>
  );
};
