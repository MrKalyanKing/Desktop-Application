export interface OcrResult {
  text: string;
  contentType: string;
  language: string;
  confidence: number;
  width: number;
  height: number;
  captureTimeMs: number;
}

export type ScreenshotMode = 'active' | 'full' | 'region' | 'scroll';

export type ScreenshotStep = 'idle' | 'capturing' | 'ocr' | 'ai' | 'done' | 'error';

export interface ScreenshotState {
  isCapturing: boolean;
  isProcessing: boolean;
  step: ScreenshotStep;
  currentPreview: string | null; // base64 URL
  lastOcrResult: OcrResult | null;
  error: string | null;
  mode: ScreenshotMode | null;
}
