import React, { useState } from 'react';
import {
  Settings24Regular,
  Bot24Regular,
  Mic24Regular,
  Square24Regular,
  Keyboard24Regular,
  Dismiss24Regular,
  Checkmark24Filled,
  Apps24Regular,
} from '@fluentui/react-icons';
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

const TABS: { id: TabType; label: string; icon: React.ReactNode }[] = [
  { id: 'general', label: 'General', icon: <Apps24Regular style={{ fontSize: 14 }} /> },
  { id: 'ai', label: 'AI', icon: <Bot24Regular style={{ fontSize: 14 }} /> },
  { id: 'voice', label: 'Voice', icon: <Mic24Regular style={{ fontSize: 14 }} /> },
  { id: 'widget', label: 'Widget', icon: <Square24Regular style={{ fontSize: 14 }} /> },
  { id: 'hotkeys', label: 'Keys', icon: <Keyboard24Regular style={{ fontSize: 14 }} /> },
];

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
      <div className="flex-1 flex items-center justify-center text-[13px] text-slate-400">
        Loading settings…
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
    <div className="flex-1 min-h-0 w-full flex flex-col overflow-hidden">
      <div className="shrink-0 px-3 pt-2 pb-2 border-b border-white/10">
        <div className="flex items-center gap-2 mb-2">
          <Settings24Regular className="text-cyan-300" style={{ fontSize: 18 }} />
          <span className="text-[15px] font-semibold text-slate-100">Settings</span>
        </div>
        <div className="flex gap-1 overflow-x-auto">
          {TABS.map((tab) => (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveTab(tab.id)}
              className={`flex items-center gap-1 px-2.5 py-2 rounded-lg text-[12px] font-semibold whitespace-nowrap ${
                activeTab === tab.id
                  ? 'bg-cyan-400/15 text-cyan-200 ring-1 ring-cyan-400/25'
                  : 'text-slate-400 hover:bg-white/5 hover:text-slate-200'
              }`}
            >
              {tab.icon}
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto px-3 py-3 w-full">
        {renderTabContent()}
      </div>

      <div className="shrink-0 flex justify-end gap-2 px-3 py-2.5 border-t border-white/10 bg-[rgba(12,16,32,0.98)]">
        <button
          type="button"
          onClick={onClose}
          className="inline-flex items-center gap-1 px-3 py-2 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 text-[12px] text-slate-300 font-semibold"
        >
          <Dismiss24Regular style={{ fontSize: 14 }} />
          Cancel
        </button>
        <button
          type="button"
          disabled={loading}
          onClick={handleSave}
          className="inline-flex items-center gap-1 px-4 py-2 rounded-lg bg-cyan-400/20 hover:bg-cyan-400/30 border border-cyan-400/30 text-[12px] text-cyan-100 font-semibold disabled:opacity-50"
        >
          <Checkmark24Filled style={{ fontSize: 14 }} />
          {loading ? 'Saving…' : 'Save'}
        </button>
      </div>
    </div>
  );
};
