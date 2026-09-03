import { useDispatch, useSelector } from 'react-redux';
import { invoke } from '@tauri-apps/api/core';
import { 
  setCapturing, 
  setProcessing, 
  setStep, 
  setPreview, 
  setOcrResult, 
  setError, 
  setMode 
} from '../stores/screenshot.store';
import { screenshotService } from '../services/screenshot.service';
import { useAI } from '../../ai';

function asErrorMessage(err: unknown, fallback: string): string {
  if (typeof err === 'string' && err.trim()) return err;
  if (err && typeof err === 'object') {
    const o = err as Record<string, unknown>;
    if (typeof o.message === 'string' && o.message.trim()) return o.message;
  }
  return fallback;
}

export const useScreenshot = () => {
  const dispatch = useDispatch();
  const isCapturing = useSelector((state: any) => state.screenshot.isCapturing);
  const isProcessing = useSelector((state: any) => state.screenshot.isProcessing);
  const step = useSelector((state: any) => state.screenshot.step);
  const currentPreview = useSelector((state: any) => state.screenshot.currentPreview);
  const lastOcrResult = useSelector((state: any) => state.screenshot.lastOcrResult);
  const error = useSelector((state: any) => state.screenshot.error);
  const mode = useSelector((state: any) => state.screenshot.mode);

  const { presentVoiceAnswer } = useAI();

  const handleCaptureResult = async (base64Img: string) => {
    dispatch(setPreview(base64Img));
    dispatch(setCapturing(false));
    dispatch(setProcessing(true));
    dispatch(setStep('ai'));

    try {
      const answer = await invoke<string>('analyze_screenshot_cmd', {
        imageBase64: base64Img,
      });
      if (answer?.trim()) {
        await presentVoiceAnswer(answer.trim(), 'OCR');
      }
      dispatch(setProcessing(false));
      dispatch(setStep('done'));
    } catch (err: unknown) {
      console.error('Screenshot analyze failed:', err);
      dispatch(setError(asErrorMessage(err, 'Could not analyze screenshot')));
      dispatch(setProcessing(false));
    }
  };

  const runCapture = async (mode: 'active' | 'full' | 'region' | 'scroll', fn: () => Promise<string>) => {
    if (isCapturing || isProcessing) return;
    dispatch(setError(null));
    dispatch(setPreview(null));
    dispatch(setOcrResult(null));
    dispatch(setMode(mode));
    dispatch(setCapturing(true));
    dispatch(setStep('capturing'));

    try {
      const img = await Promise.race([
        fn(),
        new Promise<string>((_, reject) =>
          setTimeout(() => reject(new Error('Screenshot timed out')), 10000)
        ),
      ]);
      await handleCaptureResult(img);
    } catch (err: unknown) {
      dispatch(setError(asErrorMessage(err, 'Screenshot capture failed')));
      dispatch(setCapturing(false));
      dispatch(setProcessing(false));
    }
  };

  const captureActiveWindow = () => runCapture('active', screenshotService.captureActiveWindow);
  const captureFullscreen = () => runCapture('full', screenshotService.captureFullscreen);
  const captureRegion = (x: number, y: number, w: number, h: number) =>
    runCapture('region', () => screenshotService.captureRegion(x, y, w, h));
  const startScrollCapture = () => runCapture('scroll', screenshotService.startScrollCapture);

  const openRegionSelector = async () => {
    try {
      await screenshotService.openRegionSelector();
    } catch (err: unknown) {
      dispatch(setError(asErrorMessage(err, 'Failed to open region selector')));
    }
  };

  return {
    isCapturing,
    isProcessing,
    step,
    currentPreview,
    lastOcrResult,
    error,
    mode,
    captureActiveWindow,
    captureFullscreen,
    captureRegion,
    startScrollCapture,
    openRegionSelector,
  };
};
