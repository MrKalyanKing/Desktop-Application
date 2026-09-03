import React from 'react';
import {
  Mic24Filled,
  Speaker224Regular,
  Flash24Regular,
} from '@fluentui/react-icons';
import type { CaptureMode } from '../hooks/useVoiceAgent';

interface ModeToggleProps {
  mode: CaptureMode;
  onChange: (mode: CaptureMode) => void;
  disabled?: boolean;
}

export const ModeToggle: React.FC<ModeToggleProps> = ({ mode, onChange, disabled }) => {
  const items: { id: CaptureMode; label: string; title: string; icon: React.ReactNode }[] = [
    { id: 'both', label: 'Both', title: 'Microphone and system audio', icon: <Flash24Regular style={{ fontSize: 8 }} /> },
    { id: 'mic', label: 'Mic', title: 'Your microphone only', icon: <Mic24Filled style={{ fontSize: 8 }} /> },
    { id: 'system', label: 'System', title: 'Meet / Teams / speakers', icon: <Speaker224Regular style={{ fontSize: 8 }} /> },
  ];

  return (
    <div className="flex flex-col gap-1 w-full">
      <div className="flex bg-black/25 border border-white/8 rounded-lg p-px select-none text-[9px] font-medium w-full">
        {items.map((item) => (
          <button
            key={item.id}
            type="button"
            onClick={() => onChange(item.id)}
            disabled={disabled}
            className={`flex-1 flex items-center justify-center gap-0.5 py-1 px-0.5 rounded-md transition-all cursor-pointer disabled:opacity-40 ${
              mode === item.id
                ? 'bg-cyan-400/15 text-cyan-200 shadow-sm ring-1 ring-cyan-400/25'
                : 'text-slate-400 hover:text-slate-200'
            }`}
            title={item.title}
          >
            {item.icon}
            {item.label}
          </button>
        ))}
      </div>
    </div>
  );
};
