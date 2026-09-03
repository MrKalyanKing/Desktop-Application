import { buildContextString } from './contextBuilder';
import { historyManager } from './historyManager';
import { memoryManager } from './memoryManager';
import { SYSTEM_PROMPT } from '../services/prompt.service';
import type { PromptContext, ChatMessage } from '../types/ai.types';

export const conversationService = {
  orchestrateRequest: async (
    prompt: string,
    _source: string,
    historyLimit = 4
  ): Promise<{ fullPrompt: string; systemPrompt: string }> => {
    const history = await historyManager.loadHistory(historyLimit);
    const activeContext = memoryManager.getContext();
    const contextWithHistory: PromptContext = {
      ...activeContext,
      chatHistory: history,
    };

    const fullPrompt = buildContextString(prompt, contextWithHistory);

    return {
      fullPrompt,
      systemPrompt: SYSTEM_PROMPT,
    };
  },

  saveMessage: async (role: 'user' | 'assistant', content: string, source: string): Promise<void> => {
    await historyManager.saveMessage(role, content, source);
  },

  loadHistory: async (limit?: number): Promise<ChatMessage[]> => {
    return await historyManager.loadHistory(limit);
  },

  clearHistory: async (): Promise<void> => {
    await historyManager.clearHistory();
    memoryManager.clearContext();
  }
};
