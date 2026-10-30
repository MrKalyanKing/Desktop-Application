import React from 'react';
import type { AppSettings } from '../types/settings.types';
import { GlassToggle } from '../../../shared/components/glass';

interface SubProps {
  preferences: AppSettings;
  onChange: (pref: AppSettings) => void;
}

export const GeneralSettings: React.FC<SubProps> = ({ preferences, onChange }) => {
  return (
    <div className="ks-settings-page">
      <div className="lg-stack">
        <div className="lg-row">
          <span className="lg-row-label">Launch on startup</span>
          <GlassToggle
            checked={preferences.launchOnStartup || false}
            onChange={(checked) => onChange({ ...preferences, launchOnStartup: checked })}
          />
        </div>
        <div className="lg-row">
          <span className="lg-row-label">Start minimized</span>
          <GlassToggle
            checked={preferences.startupMinimized || false}
            onChange={(checked) => onChange({ ...preferences, startupMinimized: checked })}
          />
        </div>
      </div>
    </div>
  );
};
