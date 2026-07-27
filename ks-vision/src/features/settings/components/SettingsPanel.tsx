import React, { useState } from 'react';
import { useSettings } from '../hooks/useSettings';
import { GeneralSettings } from './GeneralSettings';
import { AISettingsView } from './AISettings';
import { VoiceSettingsView } from './VoiceSettings';
import { WidgetSettingsView } from './WidgetSettings';
import { HotkeySettingsView } from './HotkeySettings';
import type { AppSettings } from '../types/settings.types';

interface SettingsPanelProps {
  onClose: () => void;
}

type TabType = 'general' | 'ai' | 'voice' | 'widget' | 'hotkeys';

export const SettingsPanel: React.FC<SettingsPanelProps> = ({ onClose }) => {
  const { preferences, saveSettings, loading } = useSettings();
  const [activeTab, setActiveTab] = useState<TabType>('general');
  const [editedPrefs, setEditedPrefs] = useState<AppSettings | null>(null);

  React.useEffect(() => {
    if (preferences && !editedPrefs) {
      setEditedPrefs(preferences);
    }
  }, [preferences]);

  if (!editedPrefs) {
    return (
      <div className="flex-1 flex items-center justify-center text-[10px] text-slate-400 select-none">
        Loading settings...
      </div>
    );
  }

  const handleSave = async () => {
    await saveSettings(editedPrefs);
    onClose();
  };

  const renderTabContent = () => {
    switch (activeTab) {
      case 'general':
        return <GeneralSettings preferences={editedPrefs} onChange={setEditedPrefs} />;
      case 'ai':
        return <AISettingsView preferences={editedPrefs} onChange={setEditedPrefs} />;
      case 'voice':
        return <VoiceSettingsView preferences={editedPrefs} onChange={setEditedPrefs} />;
      case 'widget':
        return <WidgetSettingsView preferences={editedPrefs} onChange={setEditedPrefs} />;
      case 'hotkeys':
        return <HotkeySettingsView preferences={editedPrefs} onChange={setEditedPrefs} />;
    }
  };

  return (
    <div className="flex-1 flex flex-col justify-between text-xs bg-slate-900/40 p-2 min-h-0 select-none">
      {/* Tab headers */}
      <div className="flex gap-1 border-b border-slate-800/40 pb-1 mb-1">
        {(['general', 'ai', 'voice', 'widget', 'hotkeys'] as TabType[]).map((tab) => (
          <button
            key={tab}
            type="button"
            onClick={() => setActiveTab(tab)}
            className={`px-1.5 py-0.5 rounded text-[8px] font-bold uppercase transition-all ${
              activeTab === tab 
                ? 'bg-cyan-950 text-cyan-400 border border-cyan-800/30' 
                : 'text-slate-400 hover:bg-slate-800/30'
            }`}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* Tab body */}
      <div className="flex-1 min-h-0 overflow-y-auto py-1">
        {renderTabContent()}
      </div>

      {/* Action buttons */}
      <div className="flex justify-end gap-1.5 pt-1.5 border-t border-slate-800/40 mt-1">
        <button
          type="button"
          onClick={onClose}
          className="px-2 py-0.5 bg-slate-950/50 hover:bg-slate-900 border border-slate-800/60 rounded text-[9px] text-slate-300 font-bold active:scale-95 transition-all"
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={loading}
          onClick={handleSave}
          className="px-2.5 py-0.5 bg-cyan-950 hover:bg-cyan-900 border border-cyan-800/50 rounded text-[9px] text-cyan-300 font-bold active:scale-95 transition-all disabled:opacity-50"
        >
          {loading ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  );
};
