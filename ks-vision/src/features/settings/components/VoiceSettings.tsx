import React from 'react';
import type { AppSettings } from '../types/settings.types';

interface SubProps {
  preferences: AppSettings;
  onChange: (pref: AppSettings) => void;
}

export const VoiceSettingsView: React.FC<SubProps> = ({ preferences, onChange }) => {
  const updateVoice = (key: string, value: any) => {
    onChange({
      ...preferences,
      voice: {
        ...preferences.voice,
        [key]: value,
      },
    });
  };

  return (
    <div className="ks-settings-page">
      <div className="ks-settings-card">
        <label className="ks-settings-label">Input device</label>
        <select
          value={preferences.voice.inputDevice}
          onChange={(e) => updateVoice('inputDevice', e.target.value)}
          className="ks-settings-input cursor-pointer"
        >
          <option value="default">Default system microphone</option>
        </select>

        <label className="ks-settings-label">Silence timeout (ms)</label>
        <input
          type="number"
          value={preferences.voice.silenceTimeout}
          onChange={(e) => updateVoice('silenceTimeout', parseInt(e.target.value) || 2000)}
          className="ks-settings-input"
        />

        <label className="ks-settings-label">Push-to-talk shortcut</label>
        <input
          type="text"
          value={preferences.voice.pushToTalkShortcut}
          onChange={(e) => updateVoice('pushToTalkShortcut', e.target.value)}
          className="ks-settings-input font-mono"
        />
      </div>
    </div>
  );
};
