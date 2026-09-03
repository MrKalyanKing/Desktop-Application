import React from 'react';
import type { AppSettings } from '../types/settings.types';

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
      <div className="ks-settings-card">
        <div className="grid grid-cols-2 gap-3 w-full">
          <div className="flex flex-col gap-1">
            <label className="ks-settings-label">Width (px)</label>
            <input
              type="number"
              min="200"
              max="800"
              value={preferences.widget.width}
              onChange={(e) => updateWidget('width', parseInt(e.target.value) || 300)}
              className="ks-settings-input"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="ks-settings-label">Height (px)</label>
            <input
              type="number"
              min="100"
              max="800"
              value={preferences.widget.height}
              onChange={(e) => updateWidget('height', parseInt(e.target.value) || 150)}
              className="ks-settings-input"
            />
          </div>
        </div>

        <label className="ks-settings-label">Opacity ({preferences.widget.opacity}%)</label>
        <input
          type="range"
          min="20"
          max="100"
          value={preferences.widget.opacity}
          onChange={(e) => updateWidget('opacity', parseInt(e.target.value))}
          className="accent-cyan-500 w-full h-2"
        />

        <label className="ks-settings-row cursor-pointer">
          <span>Always on top</span>
          <input
            type="checkbox"
            checked={preferences.widget.alwaysOnTop}
            onChange={(e) => updateWidget('alwaysOnTop', e.target.checked)}
            className="accent-cyan-500 h-4 w-4"
          />
        </label>

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
  );
};
