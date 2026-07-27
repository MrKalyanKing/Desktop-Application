import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';

export const widgetService = {
  hide: async (): Promise<void> => {
    try {
      const win = getCurrentWindow();
      await win.hide();
    } catch (e) {
      console.error('Failed to hide window via service:', e);
    }
  },
  show: async (): Promise<void> => {
    try {
      const win = getCurrentWindow();
      await win.show();
      await win.setFocus();
    } catch (e) {
      console.error('Failed to show window via service:', e);
    }
  },
  savePosition: async (x: number, y: number): Promise<void> => {
    try {
      await invoke('save_position', { x, y });
    } catch (e) {
      console.error('Failed to invoke save_position command:', e);
    }
  }
};
