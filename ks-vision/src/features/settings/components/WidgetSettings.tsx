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
    <div className="flex flex-col gap-1.5 text-[10px] text-slate-300">
      <div className="grid grid-cols-2 gap-1.5">
        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Width (px)</label>
          <input
            type="number"
            min="200"
            max="800"
            value={preferences.widget.width}
            onChange={(e) => updateWidget('width', parseInt(e.target.value) || 300)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 min-w-0"
          />
        </div>
        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Height (px)</label>
          <input
            type="number"
            min="100"
            max="600"
            value={preferences.widget.height}
            onChange={(e) => updateWidget('height', parseInt(e.target.value) || 150)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 min-w-0"
          />
        </div>
      </div>

      <div className="flex flex-col gap-0.5">
        <label className="text-slate-400 font-bold">Opacity ({preferences.widget.opacity}%)</label>
        <input
          type="range"
          min="20"
          max="100"
          value={preferences.widget.opacity}
          onChange={(e) => updateWidget('opacity', parseInt(e.target.value))}
          className="accent-cyan-500 cursor-pointer h-1.5"
        />
      </div>

      <label className="flex items-center justify-between cursor-pointer p-1 hover:bg-slate-800/20 rounded select-none">
        <span>Always On Top</span>
        <input 
          type="checkbox"
          checked={preferences.widget.alwaysOnTop}
          onChange={(e) => updateWidget('alwaysOnTop', e.target.checked)}
          className="accent-cyan-500 h-3.5 w-3.5"
        />
      </label>

      <div className="flex items-center justify-between gap-1">
        <span>Theme</span>
        <select
          value={preferences.widget.theme}
          onChange={(e) => updateWidget('theme', e.target.value)}
          className="bg-slate-950/80 border border-slate-800/60 rounded px-1.5 py-0.5 text-slate-200 outline-none cursor-pointer"
        >
          <option value="dark">Sleek Dark Mode</option>
          <option value="light">Balanced Light Mode</option>
        </select>
      </div>
    </div>
  );
};
