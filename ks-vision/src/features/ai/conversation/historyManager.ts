import { invoke } from '@tauri-apps/api/core';
import type { ChatMessage } from '../types/ai.types';

export interface DbMessageRecord {
  id: number;
  role: string;
  content: string;
  timestamp: number;
  source: string;
}

export const historyManager = {
  saveMessage: async (role: string, content: string, source: string): Promise<void> => {
    try {
      await invoke('db_save_message_cmd', { role, content, source });
    } catch (err) {
      console.error('Failed to save message to SQLite database:', err);
    }
  },

  loadHistory: async (limit = 50): Promise<ChatMessage[]> => {
    try {
      const records = await invoke<DbMessageRecord[]>('db_load_history_cmd', { limit });
      return records.map(rec => ({
        id: rec.id,
        role: rec.role as 'user' | 'assistant',
        content: rec.content,
        timestamp: rec.timestamp * 1000,
        source: rec.source || undefined,
      }));
    } catch (err) {
      console.error('Failed to load conversation log from SQLite database:', err);
      return [];
    }
  },

  clearHistory: async (): Promise<void> => {
    try {
      await invoke('db_clear_history_cmd');
    } catch (err) {
      console.error('Failed to wipe conversation log from SQLite database:', err);
    }
  },
};
