import React from 'react';
import type { AppSettings } from '../types/settings.types';

interface SubProps {
  preferences: AppSettings;
  onChange: (pref: AppSettings) => void;
}

export const GeneralSettings: React.FC<SubProps> = ({ preferences, onChange }) => {
  return (
    <div className="flex flex-col gap-2 text-[10px] text-slate-300">
      <label className="flex items-center justify-between gap-2 cursor-pointer p-1.5 hover:bg-slate-800/25 rounded-md transition-all select-none">
        <span>Launch on Startup</span>
        <input 
          type="checkbox"
          checked={preferences.launchOnStartup || false}
          onChange={(e) => onChange({ ...preferences, launchOnStartup: e.target.checked })}
          className="accent-cyan-500 h-3.5 w-3.5 rounded border-slate-800 cursor-pointer"
        />
      </label>
      <label className="flex items-center justify-between gap-2 cursor-pointer p-1.5 hover:bg-slate-800/25 rounded-md transition-all select-none">
        <span>Start Minimized</span>
        <input 
          type="checkbox"
          checked={preferences.startupMinimized || false}
          onChange={(e) => onChange({ ...preferences, startupMinimized: e.target.checked })}
          className="accent-cyan-500 h-3.5 w-3.5 rounded border-slate-800 cursor-pointer"
        />
      </label>
    </div>
  );
};
