import type { PromptContext } from '../types/ai.types';

export class MemoryManager {
  private activeContext: PromptContext = {};

  public setOcrText(text: string) {
    this.activeContext.ocrText = text;
  }

  public setVoiceText(text: string) {
    this.activeContext.voiceText = text;
  }

  public getContext(): PromptContext {
    return this.activeContext;
  }

  public clearContext() {
    this.activeContext = {};
  }
}

export const memoryManager = new MemoryManager();
