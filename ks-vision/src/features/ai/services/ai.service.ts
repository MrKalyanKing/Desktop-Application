import { geminiService } from './gemini.service';
import type { HealthStatus, ModelsListResponse, AIError } from '../types/ai.types';

export const aiService = {
  checkAvailability: async (url?: string): Promise<HealthStatus> => {
    return await geminiService.healthCheck(url);
  },

  fetchModels: async (url?: string): Promise<ModelsListResponse> => {
    return await geminiService.getModels(url);
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
      return await geminiService.askAI(requestId, model, prompt, system, options, url);
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
      await geminiService.streamAI(requestId, model, prompt, system, options, url, onChunk);
    } catch (err: any) {
      throw aiService.mapError(err);
    }
  },

  cancel: async (requestId: string): Promise<void> => {
    try {
      await geminiService.cancelAI(requestId);
    } catch (err: any) {
      console.error('Failed to cancel running generation request:', err);
    }
  },

  mapError: (error: any): AIError => {
    const msg = typeof error === 'string' ? error : error.message || JSON.stringify(error);
    if (
      msg.includes('unavailable') ||
      msg.includes('Connection failed') ||
      msg.includes('API Key is not set') ||
      msg.includes('API key is missing')
    ) {
      return {
        type: 'SERVER_UNAVAILABLE',
        message:
          'Gemini API is not configured. Add GEMINI_API_KEY to your .env file and restart the app.',
      };
    }
    if (msg.includes('not found') || msg.includes('ModelNotFound') || msg.includes('Failed to switch model')) {
      return {
        type: 'MODEL_NOT_FOUND',
        message:
          'Gemini model unavailable or exhausted. Try another model in Settings, or wait for quota reset.',
      };
    }
    if (msg.includes('cancelled') || msg.includes('RequestCancelled')) {
      return {
        type: 'REQUEST_CANCELLED',
        message: 'AI request generation cancelled.',
      };
    }
    return {
      type: 'GENERAL_ERROR',
      message: msg,
    };
  },
};
