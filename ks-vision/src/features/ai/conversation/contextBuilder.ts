import type { PromptContext } from '../types/ai.types';

export const buildContextString = (prompt: string, context: PromptContext): string => {
  let contextString = '';

  if (context.ocrText && context.ocrText.trim().length > 0) {
    contextString += `### Context (Screen OCR):\n${context.ocrText.trim()}\n\n`;
  }

  if (context.voiceText && context.voiceText.trim().length > 0) {
    contextString += `### Context (Voice Transcription):\n${context.voiceText.trim()}\n\n`;
  }

  if (context.chatHistory && context.chatHistory.length > 0) {
    contextString += `### Context (Recent History):\n`;
    context.chatHistory.forEach(msg => {
      const sourceTag = msg.source ? ` [via ${msg.source}]` : '';
      contextString += `${msg.role.toUpperCase()}${sourceTag}: ${msg.content}\n`;
    });
    contextString += `\n`;
  }

  if (prompt && prompt.trim().length > 0) {
    contextString += `### User Query:\n${prompt.trim()}`;
  }

  return contextString.trim();
};
