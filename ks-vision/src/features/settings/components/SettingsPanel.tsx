import React, { useState } from 'react';
import { AnimatePresence, LayoutGroup, motion } from 'motion/react';
import {
  Settings24Regular,
  Bot24Regular,
  Mic24Regular,
  Square24Regular,
  Keyboard24Regular,
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
  { id: 'general', label: 'General', icon: <Apps24Regular style={{ fontSize: 13 }} /> },
  { id: 'ai', label: 'AI', icon: <Bot24Regular style={{ fontSize: 13 }} /> },
  { id: 'voice', label: 'Voice', icon: <Mic24Regular style={{ fontSize: 13 }} /> },
  { id: 'widget', label: 'Widget', icon: <Square24Regular style={{ fontSize: 13 }} /> },
  { id: 'hotkeys', label: 'Keys', icon: <Keyboard24Regular style={{ fontSize: 13 }} /> },
];

const FOOTER_STATUS: Record<TabType, string> = {
  general: 'Preferences',
  ai: 'Model ready',
  voice: 'Voice ready',
  widget: 'Overlay ready',
  hotkeys: 'Shortcuts active',
};

const glassTransition = {
  duration: 0.42,
  ease: [0.22, 1, 0.36, 1] as const,
};

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
      <div className="flex flex-1 items-center justify-center text-[13px] text-[var(--lg-text-secondary)]">
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
    <div className="flex min-h-0 w-full flex-1 flex-col overflow-hidden">
      <div className="shrink-0 px-4 pb-2">
        <div className="mb-3 flex items-center gap-2">
          <Settings24Regular className="text-[var(--lg-text)]" style={{ fontSize: 18 }} />
          <span className="text-[18px] font-semibold tracking-[-0.03em] text-white">Settings</span>
        </div>
        <LayoutGroup>
          <div className="lg-tabs">
            {TABS.map((tab) => (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActiveTab(tab.id)}
                className={activeTab === tab.id ? 'is-active' : ''}
              >
                {activeTab === tab.id && (
                  <motion.span
                    layoutId="settings-tab-glass"
                    className="lg-tab-glass"
                    transition={{ type: 'spring', stiffness: 420, damping: 36 }}
                  />
                )}
                <span className="lg-tab-label">
                  {tab.icon}
                  {tab.label}
                </span>
              </button>
            ))}
          </div>
        </LayoutGroup>
      </div>

      <div className="lg-settings-scroll">
        <AnimatePresence mode="wait">
          <motion.div
            key={activeTab}
            initial={{ opacity: 0, filter: 'blur(16px)', y: 14, scale: 0.97 }}
            animate={{ opacity: 1, filter: 'blur(0px)', y: 0, scale: 1 }}
            exit={{ opacity: 0, filter: 'blur(12px)', y: -10, scale: 0.98 }}
            transition={glassTransition}
          >
            {renderTabContent()}
          </motion.div>
        </AnimatePresence>
      </div>

      <div className="lg-footer">
        <div className="lg-status">
          <span className="lg-status-dot" />
          {FOOTER_STATUS[activeTab]}
        </div>
        <div className="flex items-center gap-2">
          <button type="button" onClick={onClose} className="lg-btn">
            Cancel
          </button>
          <button type="button" disabled={loading} onClick={handleSave} className="lg-btn lg-btn--primary">
            {loading ? 'Saving…' : 'Save'}
          </button>
        </div>
      </div>
    </div>
  );
};
