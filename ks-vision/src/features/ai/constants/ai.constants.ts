export const OLLAMA_DEFAULT_URL = 'http://localhost:11434';

export const DEFAULT_MODEL = 'gemini-3.5-flash-lite';

export const STORAGE_KEYS = {
  ACTIVE_MODEL: 'ks_vision_active_model',
  OLLAMA_URL: 'ks_vision_ollama_url',
};

export const AI_STATUS = {
  DISCONNECTED: 'disconnected',
  CONNECTED: 'connected',
  ERROR: 'error',
} as const;
