import { invoke } from '@tauri-apps/api/core';
import type { OcrResult } from '../types/screenshot.types';

export const screenshotService = {
  captureActiveWindow: async (): Promise<string> => {
    return await invoke<string>('capture_active_window_cmd');
  },

  captureFullscreen: async (): Promise<string> => {
    return await invoke<string>('capture_fullscreen_cmd');
  },

  captureRegion: async (x: number, y: number, w: number, h: number): Promise<string> => {
    return await invoke<string>('capture_region_cmd', { x, y, w, h });
  },

  startScrollCapture: async (): Promise<string> => {
    return await invoke<string>('start_scroll_capture_cmd');
  },

  openRegionSelector: async (): Promise<void> => {
    return await invoke<void>('open_region_selector_cmd');
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
