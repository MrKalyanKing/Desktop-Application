import React, { useState } from 'react';
import type { AppSettings } from '../types/settings.types';

interface SubProps {
  preferences: AppSettings;
  onChange: (pref: AppSettings) => void;
}

export const HotkeySettingsView: React.FC<SubProps> = ({ preferences, onChange }) => {
  const [duplicateError, setDuplicateError] = useState<string | null>(null);

  const updateHotkey = (key: string, value: string) => {
    const allKeys = Object.entries(preferences.hotkeys).filter(([k]) => k !== key);
    const isDuplicate = allKeys.some(([_, val]) => val.toLowerCase() === value.toLowerCase());

    if (isDuplicate && value.trim() !== '') {
      setDuplicateError(`Shortcut "${value}" is already mapped.`);
    } else {
      setDuplicateError(null);
    }

    onChange({
      ...preferences,
      hotkeys: {
        ...preferences.hotkeys,
        [key]: value,
      },
    });
  };

  const fields: { key: keyof AppSettings['hotkeys']; label: string }[] = [
    { key: 'voice', label: 'Voice input' },
    { key: 'screenshot', label: 'Screenshot' },
    { key: 'regionCapture', label: 'Region crop' },
    { key: 'fullScreen', label: 'Fullscreen' },
    { key: 'scrollCapture', label: 'Scroll capture' },
    { key: 'toggleWidget', label: 'Toggle widget' },
    { key: 'emergencyHide', label: 'Emergency hide' },
  ];

  return (
    <div className="ks-settings-page">
      {duplicateError && (
        <div className="text-rose-300 font-semibold bg-rose-950/30 border border-rose-800/40 p-2.5 rounded-xl text-[12px]">
          {duplicateError}
        </div>
      )}
      <div className="ks-settings-card">
        {fields.map((f) => (
          <div key={f.key} className="flex flex-col gap-1 w-full">
            <label className="ks-settings-label">{f.label}</label>
            <input
              type="text"
              value={preferences.hotkeys[f.key]}
              onChange={(e) => updateHotkey(f.key, e.target.value)}
              className="ks-settings-input font-mono"
            />
          </div>
        ))}
      </div>
    </div>
  );
};
