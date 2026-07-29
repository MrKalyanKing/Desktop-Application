export type AIStatusType = 'disconnected' | 'connected' | 'error';

export interface AiModel {
  name: string;
  model: string;
  size: number;
  digest: string;
}

export type ModelsListResponse = {
  models: AiModel[];
};

export interface HealthStatus {
  available: boolean;
  url: string;
  message: string;
}

export interface ChatMessage {
  id?: number;
  role: 'user' | 'assistant' | 'system';
  content: string;
  timestamp: number;
  source?: string;
}

export interface PromptContext {
  systemPrompt?: string;
  contextText?: string;
  ocrText?: string;
  voiceText?: string;
  chatHistory?: ChatMessage[];
}

export interface AIError {
  type: string;
  message: string;
}
