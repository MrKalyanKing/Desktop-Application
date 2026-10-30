import React, { useState } from 'react';
import type { AppSettings } from '../types/settings.types';
import { GlassHotkeyField } from '../../../shared/components/glass';

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
        <div className="lg-row text-[12px] text-[var(--lg-danger)]">{duplicateError}</div>
      )}
      <div className="lg-stack">
        {fields.map((f) => (
          <div key={f.key} className="lg-row">
            <span className="lg-row-label">{f.label}</span>
            <GlassHotkeyField
              value={preferences.hotkeys[f.key]}
              onChange={(value) => updateHotkey(f.key, value)}
            />
          </div>
        ))}
      </div>
    </div>
  );
};
