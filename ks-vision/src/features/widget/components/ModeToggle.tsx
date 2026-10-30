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
    { id: 'both', label: 'Both', title: 'Microphone and system audio', icon: <Flash24Regular style={{ fontSize: 11 }} /> },
    { id: 'mic', label: 'Mic', title: 'Your microphone only', icon: <Mic24Filled style={{ fontSize: 11 }} /> },
    { id: 'system', label: 'System', title: 'Meet / Teams / speakers', icon: <Speaker224Regular style={{ fontSize: 11 }} /> },
  ];
  const activeIndex = Math.max(0, items.findIndex((item) => item.id === mode));

  return (
    <div
      className="lg-segmented w-full"
      style={{ ['--lg-seg-count' as string]: items.length, ['--lg-seg-index' as string]: activeIndex }}
    >
      <div className="lg-segmented__pill" />
      {items.map((item) => (
        <button
          key={item.id}
          type="button"
          onClick={() => onChange(item.id)}
          disabled={disabled}
          className={mode === item.id ? 'is-active' : ''}
          title={item.title}
        >
          {item.icon}
          {item.label}
        </button>
      ))}
    </div>
  );
};
