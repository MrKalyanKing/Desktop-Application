import React from 'react';

interface ModeToggleProps {
  mode: 'mic' | 'system';
  onChange: (mode: 'mic' | 'system') => void;
  disabled?: boolean;
}

export const ModeToggle: React.FC<ModeToggleProps> = ({ mode, onChange, disabled }) => {
  return (
    <div className="flex bg-slate-950/40 border border-slate-800/40 rounded-md p-0.5 select-none text-[9px] font-bold w-full">
      <button
        type="button"
        onClick={() => onChange('mic')}
        disabled={disabled}
        className={`flex-1 flex items-center justify-center gap-1 py-1 px-2 rounded-sm transition-all cursor-pointer ${
          mode === 'mic'
            ? 'bg-cyan-950/80 text-cyan-300 border border-cyan-800/30'
            : 'text-slate-400 hover:text-slate-200'
        }`}
        title="Capture microphone (your voice only)"
      >
        🎤 My Voice
      </button>
      <button
        type="button"
        onClick={() => onChange('system')}
        disabled={disabled}
        className={`flex-1 flex items-center justify-center gap-1 py-1 px-2 rounded-sm transition-all cursor-pointer ${
          mode === 'system'
            ? 'bg-cyan-950/80 text-cyan-300 border border-cyan-800/30'
            : 'text-slate-400 hover:text-slate-200'
        }`}
        title="Capture system audio (meetings, videos, etc.)"
      >
        🔊 System Audio
      </button>
    </div>
  );
};
