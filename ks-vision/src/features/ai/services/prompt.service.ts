import { buildPrompt } from '../utils/promptBuilder';
import type { PromptContext } from '../types/ai.types';

export const SYSTEM_PROMPT = `You are an AI Meeting Copilot for software engineers.

Provide concise and accurate explanations that can be read during live meetings.

Rules:
- Answer directly.
- Focus on the user's actual question.
- Explain the purpose before implementation details.
- Keep responses under 80 words whenever possible.
- Use short paragraphs or bullet points.
- Avoid unnecessary technical theory.
- Never write long tutorials unless explicitly requested.
- If code is provided, explain what it does, why it exists, and any important logic.
- If an error is provided, explain the cause and the quickest fix.
- If the answer is lengthy, summarize the important points first.`;

export const promptService = {
  createOcrSummaryPrompt: (ocrText: string): { prompt: string; system: string } => {
    const system = 'You are an advanced screen content analyzer. Extract key text, user actions, and explain the current workflow.';
    const prompt = 'Please summarize the text and interface elements observed in this OCR capture.';
    return {
      prompt: buildPrompt(prompt, { ocrText }),
      system,
    };
  },

  createVoiceSummaryPrompt: (transcription: string): { prompt: string; system: string } => {
    const system = 'You are an advanced meeting copilot. Extract key decisions, action items, and meeting milestones from transcription logs.';
    const prompt = 'Please analyze this voice transcription and generate bulleted summaries.';
    return {
      prompt: buildPrompt(prompt, { voiceText: transcription }),
      system,
    };
  },

  createGeneralChatPrompt: (
    userMessage: string, 
    context: PromptContext,
    system = SYSTEM_PROMPT
  ): { prompt: string; system: string } => {
    return {
      prompt: buildPrompt(userMessage, context),
      system,
    };
  }
};
