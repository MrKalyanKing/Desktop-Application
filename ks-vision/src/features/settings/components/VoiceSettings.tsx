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
    <div className="flex flex-col gap-1.5 text-[10px] text-slate-300">
      <div className="flex flex-col gap-0.5">
        <label className="text-slate-400 font-bold">Input Device</label>
        <select
          value={preferences.voice.inputDevice}
          onChange={(e) => updateVoice('inputDevice', e.target.value)}
          className="bg-slate-950/80 border border-slate-800/60 rounded px-1.5 py-0.5 text-slate-200 focus:border-cyan-500/50 outline-none cursor-pointer"
        >
          <option value="default">Default System Microphone</option>
        </select>
      </div>

      <div className="grid grid-cols-2 gap-1.5">
        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Silence Timeout (ms)</label>
          <input
            type="number"
            value={preferences.voice.silenceTimeout}
            onChange={(e) => updateVoice('silenceTimeout', parseInt(e.target.value) || 2000)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 min-w-0"
          />
        </div>
        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Whisper Model</label>
          <select
            value={preferences.voice.whisperModel}
            onChange={(e) => updateVoice('whisperModel', e.target.value)}
            className="bg-slate-950/80 border border-slate-800/60 rounded px-1.5 py-0.5 text-slate-200 outline-none cursor-pointer"
          >
            <option value="tiny">Tiny (Fastest)</option>
            <option value="base">Base (Balanced)</option>
            <option value="small">Small (Accurate)</option>
          </select>
        </div>
      </div>

      <div className="flex flex-col gap-0.5">
        <label className="text-slate-400 font-bold">PTT Shortcut</label>
        <input
          type="text"
          value={preferences.voice.pushToTalkShortcut}
          onChange={(e) => updateVoice('pushToTalkShortcut', e.target.value)}
          className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 min-w-0 font-mono text-[9px]"
        />
      </div>
    </div>
  );
};
