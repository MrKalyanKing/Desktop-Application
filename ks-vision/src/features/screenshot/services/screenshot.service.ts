import { invoke } from '@tauri-apps/api/core';
import type { OcrResult } from '../types/screenshot.types';

export interface ScrollCaptureResult {
  image: string;
  frames: number;
  status: 'ended' | 'partial';
  reason: string;
}

export const screenshotService = {
  captureActiveWindow: async (targetHwnd?: number | null): Promise<string> => {
    return await invoke<string>('capture_active_window_cmd', { targetHwnd: targetHwnd ?? null });
  },

  captureFullscreen: async (): Promise<string> => {
    return await invoke<string>('capture_fullscreen_cmd');
  },

  /** Physical screen coordinates. */
  captureRegion: async (x: number, y: number, w: number, h: number): Promise<string> => {
    return await invoke<string>('capture_region_cmd', { x, y, w, h });
  },

  startScrollCapture: async (targetHwnd: number, opId: number): Promise<ScrollCaptureResult> => {
    return await invoke<ScrollCaptureResult>('start_scroll_capture_cmd', { targetHwnd, opId });
  },

  openRegionSelector: async (): Promise<void> => {
    return await invoke<void>('open_region_selector_cmd');
  },

  analyze: async (imageBase64: string, opId: number | null): Promise<string> => {
    return await invoke<string>('analyze_screenshot_cmd', { imageBase64, opId });
  },

  performOcr: async (image: string, preprocess = true): Promise<OcrResult> => {
    const resp = await invoke<{
      text: string;
      content_type: string;
      language: string;
      confidence: number;
      width: number;
      height: number;
      capture_time_ms: number;
    }>('perform_ocr_cmd', { imageBase64: image, preprocess });

    return {
      text: resp.text,
      contentType: resp.content_type,
      language: resp.language,
      confidence: resp.confidence,
      width: resp.width,
      height: resp.height,
      captureTimeMs: resp.capture_time_ms,
    };
  },
};
