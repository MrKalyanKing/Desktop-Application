import type { PromptContext } from '../types/ai.types';

export const buildPrompt = (prompt: string, context: PromptContext): string => {
  let fullPrompt = '';
  
  if (context.contextText) {
    fullPrompt += `### General Context:\n${context.contextText}\n\n`;
  }
  
  if (context.ocrText) {
    fullPrompt += `### Screenshot OCR Context:\n${context.ocrText}\n\n`;
  }
  
  if (context.voiceText) {
    fullPrompt += `### Voice Transcription Context:\n${context.voiceText}\n\n`;
  }
  
  if (context.chatHistory && context.chatHistory.length > 0) {
    fullPrompt += `### Previous Conversation:\n`;
    context.chatHistory.forEach(msg => {
      fullPrompt += `${msg.role.toUpperCase()}: ${msg.content}\n`;
    });
    fullPrompt += `\n`;
  }
  
  fullPrompt += `### User Query:\n${prompt}`;
  return fullPrompt;
};
