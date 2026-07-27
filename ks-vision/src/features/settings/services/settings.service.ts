import { invoke } from '@tauri-apps/api/core';
import type { AppSettings } from '../types/settings.types';

export const settingsService = {
  loadSettings: async (): Promise<AppSettings> => {
    const raw = await invoke<string>('load_settings_cmd');
    return JSON.parse(raw);
  },

  saveSettings: async (settings: AppSettings): Promise<void> => {
    const raw = JSON.stringify(settings);
    await invoke('save_settings_cmd', { settings: raw });
    
    if (settings.launchOnStartup !== undefined) {
      await invoke('set_startup_enabled_cmd', { enabled: settings.launchOnStartup });
    }
  },

  isStartupEnabled: async (): Promise<boolean> => {
    return await invoke<boolean>('is_startup_enabled_cmd');
  },
};
