import { invoke, Channel } from '@tauri-apps/api/core';
import type { HealthStatus, ModelsListResponse } from '../types/ai.types';

export const ollamaService = {
  healthCheck: async (baseUrl?: string): Promise<HealthStatus> => {
    return await invoke<HealthStatus>('ai_health_check', { baseUrl });
  },
  
  getModels: async (baseUrl?: string): Promise<ModelsListResponse> => {
    return await invoke<ModelsListResponse>('ai_get_models', { baseUrl });
  },

  askAI: async (
    requestId: string,
    model: string,
    prompt: string,
    system?: string,
    options?: { temperature?: number; num_predict?: number },
    baseUrl?: string
  ): Promise<string> => {
    return await invoke<string>('ask_ai', {
      requestId,
      model,
      prompt,
      system,
      options,
      baseUrl,
    });
  },

  streamAI: async (
    requestId: string,
    model: string,
    prompt: string,
    system: string | undefined,
    options: { temperature?: number; num_predict?: number } | undefined,
    baseUrl: string | undefined,
    onChunk: (chunk: string) => void
  ): Promise<void> => {
    const tauriChannel = new Channel<string>((message) => {
      onChunk(message);
    });
    return await invoke<void>('stream_ai', {
      requestId,
      model,
      prompt,
      system,
      options,
      baseUrl,
      channel: tauriChannel,
    });
  },

  cancelAI: async (requestId: string): Promise<void> => {
    return await invoke<void>('cancel_ai', { requestId });
  }
};
