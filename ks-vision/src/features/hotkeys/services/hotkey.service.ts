import { register, unregister } from '@tauri-apps/plugin-global-shortcut';

export const hotkeyService = {
  registerGlobal: async (shortcut: string, handler: (event: any) => void): Promise<void> => {
    try {
      await register(shortcut, handler);
    } catch (e) {
      console.error(`Failed to register global hotkey ${shortcut}:`, e);
    }
  },
  unregisterGlobal: async (shortcut: string): Promise<void> => {
    try {
      await unregister(shortcut);
    } catch (e) {
      console.error(`Failed to unregister global hotkey ${shortcut}:`, e);
    }
  }
};
