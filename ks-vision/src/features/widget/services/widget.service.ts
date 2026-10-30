import { invoke } from '@tauri-apps/api/core';

export const widgetService = {
  // Show/hide go through Rust (Win32 show without stealing focus, and a matching hide).
  hide: async (): Promise<void> => {
    try {
      await invoke('hide_overlay_cmd');
    } catch (e) {
      console.error('Failed to hide window via service:', e);
    }
  },
  show: async (focus = true): Promise<void> => {
    try {
      await invoke('show_overlay_cmd', { focus });
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
