import React, { useState } from 'react';
import type { AppSettings } from '../types/settings.types';

interface SubProps {
  preferences: AppSettings;
  onChange: (pref: AppSettings) => void;
}

export const HotkeySettingsView: React.FC<SubProps> = ({ preferences, onChange }) => {
  const [duplicateError, setDuplicateError] = useState<string | null>(null);

  const updateHotkey = (key: string, value: string) => {
    const allKeys = Object.entries(preferences.hotkeys).filter(([k]) => k !== key);
    const isDuplicate = allKeys.some(([_, val]) => val.toLowerCase() === value.toLowerCase());

    if (isDuplicate && value.trim() !== '') {
      setDuplicateError(`Shortcut "${value}" is already mapped!`);
    } else {
      setDuplicateError(null);
    }

    onChange({
      ...preferences,
      hotkeys: {
        ...preferences.hotkeys,
        [key]: value,
      },
    });
  };

  return (
    <div className="flex flex-col gap-1 text-[10px] text-slate-300">
      {duplicateError && (
        <div className="text-rose-400 font-bold bg-rose-950/20 border border-rose-900/40 p-1 rounded text-[9px] mb-1 select-none">
          ⚠️ {duplicateError}
        </div>
      )}

      <div className="grid grid-cols-2 gap-1.5 max-h-[140px] overflow-y-auto pr-1">
        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Voice Input</label>
          <input
            type="text"
            value={preferences.hotkeys.voice}
            onChange={(e) => updateHotkey('voice', e.target.value)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 font-mono text-[9px]"
          />
        </div>

        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Screenshot</label>
          <input
            type="text"
            value={preferences.hotkeys.screenshot}
            onChange={(e) => updateHotkey('screenshot', e.target.value)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 font-mono text-[9px]"
          />
        </div>

        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Region Crop</label>
          <input
            type="text"
            value={preferences.hotkeys.regionCapture}
            onChange={(e) => updateHotkey('regionCapture', e.target.value)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 font-mono text-[9px]"
          />
        </div>

        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Fullscreen</label>
          <input
            type="text"
            value={preferences.hotkeys.fullScreen}
            onChange={(e) => updateHotkey('fullScreen', e.target.value)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 font-mono text-[9px]"
          />
        </div>

        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Scroll Capture</label>
          <input
            type="text"
            value={preferences.hotkeys.scrollCapture}
            onChange={(e) => updateHotkey('scrollCapture', e.target.value)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 font-mono text-[9px]"
          />
        </div>

        <div className="flex flex-col gap-0.5">
          <label className="text-slate-400 font-bold">Toggle Widget</label>
          <input
            type="text"
            value={preferences.hotkeys.toggleWidget}
            onChange={(e) => updateHotkey('toggleWidget', e.target.value)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 font-mono text-[9px]"
          />
        </div>

        <div className="flex flex-col gap-0.5 col-span-2">
          <label className="text-slate-400 font-bold">Emergency Hide</label>
          <input
            type="text"
            value={preferences.hotkeys.emergencyHide}
            onChange={(e) => updateHotkey('emergencyHide', e.target.value)}
            className="bg-slate-950/85 border border-slate-800/60 rounded px-1.5 py-0.5 outline-none text-slate-200 font-mono text-[9px]"
          />
        </div>
      </div>
    </div>
  );
};
