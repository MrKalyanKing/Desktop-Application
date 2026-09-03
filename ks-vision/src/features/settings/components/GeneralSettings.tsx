import React from 'react';
import type { AppSettings } from '../types/settings.types';

interface SubProps {
  preferences: AppSettings;
  onChange: (pref: AppSettings) => void;
}

export const GeneralSettings: React.FC<SubProps> = ({ preferences, onChange }) => {
  return (
    <div className="ks-settings-page">
      <div className="ks-settings-card">
        <label className="ks-settings-row cursor-pointer">
          <span>Launch on startup</span>
          <input
            type="checkbox"
            checked={preferences.launchOnStartup || false}
            onChange={(e) => onChange({ ...preferences, launchOnStartup: e.target.checked })}
            className="accent-cyan-500 h-4 w-4"
          />
        </label>
        <label className="ks-settings-row cursor-pointer">
          <span>Start minimized</span>
          <input
            type="checkbox"
            checked={preferences.startupMinimized || false}
            onChange={(e) => onChange({ ...preferences, startupMinimized: e.target.checked })}
            className="accent-cyan-500 h-4 w-4"
          />
        </label>
      </div>
    </div>
  );
};
