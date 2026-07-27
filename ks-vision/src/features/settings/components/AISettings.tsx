import React from 'react';
import type { AppSettings } from '../types/settings.types';
import { useAI } from '../../ai';

interface SubProps {
  preferences: AppSettings;
  onChange: (pref: AppSettings) => void;
}

export const AISettingsView: React.FC<SubProps> = ({ preferences, onChange }) => {
  const { models } = useAI();

  const updateAI = (key: string, value: any) => {
    onChange({
      ...preferences,
      ai: {
        ...preferences.ai,
        [key]: value,
      },
    });
  };

  return (
    <div className="flex flex-col gap-1.5 text-[10px] text-slate-300">
      <div className="flex flex-col gap-0.5">
        <label className="text-slate-400 font-bold">Active Model</label>
        <select
          value={preferences.ai.activeModel}
          onChange={(e) => updateAI('activeModel', e.target.value)}
          className="bg-slate-950/80 border border-slate-800/60 rounded px-1.5 py-0.5 text-slate-200 focus:border-cyan-500/50 outline-none cursor-pointer"
        >
          {models.length > 0 ? (
            models.map((m: any) => (
              <option key={m.name} value={m.name}>{m.name}</option>
            ))
          ) : (
            <option value={preferences.ai.activeModel}>{preferences.ai.activeModel}</option>
          )}
        </select>
      </div>

      <div className="grid grid-cols-2 gap-1.5">
        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Temp ({preferences.ai.temperature})</label>
          <input
            type="range"
            min="0"
            max="1.5"
            step="0.1"
            value={preferences.ai.temperature}
            onChange={(e) => updateAI('temperature', parseFloat(e.target.value))}
            className="accent-cyan-500 cursor-pointer h-1.5"
          />
        </div>
        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Max Tokens</label>
          <input
            type="number"
            value={preferences.ai.maxTokens}
            onChange={(e) => updateAI('maxTokens', parseInt(e.target.value) || 128)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 text-center min-w-0"
          />
        </div>
      </div>

      <label className="flex items-center justify-between cursor-pointer p-1 hover:bg-slate-800/20 rounded select-none">
        <span>Streaming Mode</span>
        <input 
          type="checkbox"
          checked={preferences.ai.streaming}
          onChange={(e) => updateAI('streaming', e.target.checked)}
          className="accent-cyan-500 h-3.5 w-3.5"
        />
      </label>

      <label className="flex items-center justify-between cursor-pointer p-1 hover:bg-slate-800/20 rounded select-none">
        <span>Auto Copy Response</span>
        <input 
          type="checkbox"
          checked={preferences.ai.autoCopyResponse}
          onChange={(e) => updateAI('autoCopyResponse', e.target.checked)}
          className="accent-cyan-500 h-3.5 w-3.5"
        />
      </label>
    </div>
  );
};
