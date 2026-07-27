import { ollamaService } from './ollama.service';
import type { HealthStatus, ModelsListResponse, AIError } from '../types/ai.types';

export const aiService = {
  checkAvailability: async (url?: string): Promise<HealthStatus> => {
    return await ollamaService.healthCheck(url);
  },

  fetchModels: async (url?: string): Promise<ModelsListResponse> => {
    return await ollamaService.getModels(url);
  },

  ask: async (
    requestId: string,
    model: string,
    prompt: string,
    system?: string,
    options?: { temperature?: number; num_predict?: number },
    url?: string
  ): Promise<string> => {
    try {
      return await ollamaService.askAI(requestId, model, prompt, system, options, url);
    } catch (err: any) {
      throw aiService.mapError(err);
    }
  },

  stream: async (
    requestId: string,
    model: string,
    prompt: string,
    system: string | undefined,
    options: { temperature?: number; num_predict?: number } | undefined,
    url: string | undefined,
    onChunk: (chunk: string) => void
  ): Promise<void> => {
    try {
      await ollamaService.streamAI(requestId, model, prompt, system, options, url, onChunk);
    } catch (err: any) {
      throw aiService.mapError(err);
    }
  },

  cancel: async (requestId: string): Promise<void> => {
    try {
      await ollamaService.cancelAI(requestId);
    } catch (err: any) {
      console.error('Failed to cancel running generation request:', err);
    }
  },

  mapError: (error: any): AIError => {
    const msg = typeof error === 'string' ? error : (error.message || JSON.stringify(error));
    if (msg.includes('unavailable') || msg.includes('Connection failed')) {
      return { 
        type: 'SERVER_UNAVAILABLE', 
        message: 'Ollama server is not running or was not found at this endpoint. Please make sure Ollama is active.' 
      };
    }
    if (msg.includes('not found') || msg.includes('ModelNotFound')) {
      return { 
        type: 'MODEL_NOT_FOUND', 
        message: 'The requested model is missing. Run "ollama pull <model>" in your shell first.' 
      };
    }
    if (msg.includes('cancelled') || msg.includes('RequestCancelled')) {
      return { 
        type: 'REQUEST_CANCELLED', 
        message: 'AI request generation cancelled.' 
      };
    }
    return { 
      type: 'GENERAL_ERROR', 
      message: msg 
    };
  }
};
