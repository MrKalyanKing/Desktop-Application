import React from 'react';
import type { AppSettings } from '../types/settings.types';
import { GlassHotkeyField } from '../../../shared/components/glass';

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
      <div className="lg-stack">
        <div className="lg-block">
          <label className="ks-settings-label">Input device</label>
          <select
            value={preferences.voice.inputDevice}
            onChange={(e) => updateVoice('inputDevice', e.target.value)}
            className="ks-settings-input cursor-pointer"
          >
            <option value="default">Default system microphone</option>
          </select>
        </div>

        <div className="lg-row">
          <span className="lg-row-label">Silence timeout (ms)</span>
          <input
            type="number"
            value={preferences.voice.silenceTimeout}
            onChange={(e) => updateVoice('silenceTimeout', parseInt(e.target.value) || 2000)}
            className="ks-settings-input"
          />
        </div>

        <div className="lg-row">
          <span className="lg-row-label">Push-to-talk shortcut</span>
          <GlassHotkeyField
            value={preferences.voice.pushToTalkShortcut}
            onChange={(value) => updateVoice('pushToTalkShortcut', value)}
          />
        </div>
      </div>
    </div>
  );
};
