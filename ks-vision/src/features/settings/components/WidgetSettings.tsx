import React from 'react';
import type { AppSettings } from '../types/settings.types';
import { GlassToggle } from '../../../shared/components/glass';

interface SubProps {
  preferences: AppSettings;
  onChange: (pref: AppSettings) => void;
}

export const WidgetSettingsView: React.FC<SubProps> = ({ preferences, onChange }) => {
  const updateWidget = (key: string, value: any) => {
    onChange({
      ...preferences,
      widget: {
        ...preferences.widget,
        [key]: value,
      },
    });
  };

  return (
    <div className="ks-settings-page">
      <div className="lg-stack">
        <div className="lg-row">
          <span className="lg-row-label">Width (px)</span>
          <input
            type="number"
            min="200"
            max="800"
            value={preferences.widget.width}
            onChange={(e) => updateWidget('width', parseInt(e.target.value) || 300)}
            className="ks-settings-input"
          />
        </div>
        <div className="lg-row">
          <span className="lg-row-label">Height (px)</span>
          <input
            type="number"
            min="100"
            max="800"
            value={preferences.widget.height}
            onChange={(e) => updateWidget('height', parseInt(e.target.value) || 150)}
            className="ks-settings-input"
          />
        </div>
        <div className="lg-block">
          <label className="ks-settings-label">Opacity ({preferences.widget.opacity}%)</label>
          <input
            type="range"
            min="20"
            max="100"
            value={preferences.widget.opacity}
            onChange={(e) => updateWidget('opacity', parseInt(e.target.value))}
            className="lg-slider"
          />
        </div>
        <div className="lg-row">
          <span className="lg-row-label">Always on top</span>
          <GlassToggle
            checked={preferences.widget.alwaysOnTop}
            onChange={(checked) => updateWidget('alwaysOnTop', checked)}
          />
        </div>
        <div className="lg-block">
          <label className="ks-settings-label">Theme</label>
          <select
            value={preferences.widget.theme}
            onChange={(e) => updateWidget('theme', e.target.value)}
            className="ks-settings-input cursor-pointer"
          >
            <option value="dark">Sleek dark</option>
            <option value="light">Balanced light</option>
          </select>
        </div>
      </div>
    </div>
  );
};
