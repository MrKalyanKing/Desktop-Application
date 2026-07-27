import { useDispatch, useSelector } from 'react-redux';
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
import { promptService } from '../../ai';

export const useScreenshot = () => {
  const dispatch = useDispatch();
  const isCapturing = useSelector((state: any) => state.screenshot.isCapturing);
  const isProcessing = useSelector((state: any) => state.screenshot.isProcessing);
  const step = useSelector((state: any) => state.screenshot.step);
  const currentPreview = useSelector((state: any) => state.screenshot.currentPreview);
  const lastOcrResult = useSelector((state: any) => state.screenshot.lastOcrResult);
  const error = useSelector((state: any) => state.screenshot.error);
  const mode = useSelector((state: any) => state.screenshot.mode);

  const { stream: streamAI } = useAI();

  const handleCaptureResult = async (base64Img: string) => {
    dispatch(setPreview(base64Img));
    dispatch(setCapturing(false));
    dispatch(setProcessing(true));
    dispatch(setStep('ocr'));

    try {
      const ocrResult = await screenshotService.performOcr(base64Img, true);
      dispatch(setOcrResult(ocrResult));
      
      dispatch(setProcessing(false));
      dispatch(setStep('ai'));

      const { prompt, system } = promptService.createOcrSummaryPrompt(ocrResult.text);

      await streamAI(prompt, { ocrText: ocrResult.text }, system);
      
      dispatch(setStep('done'));
    } catch (err: any) {
      console.error('OCR or AI explain failed:', err);
      dispatch(setError(err.message || 'Processing failed'));
      dispatch(setProcessing(false));
    }
  };

  const captureActiveWindow = async () => {
    dispatch(setError(null));
    dispatch(setPreview(null));
    dispatch(setMode('active'));
    dispatch(setCapturing(true));
    dispatch(setStep('capturing'));

    try {
      const img = await screenshotService.captureActiveWindow();
      await handleCaptureResult(img);
    } catch (err: any) {
      dispatch(setError(err.message || 'Active window capture failed'));
      dispatch(setCapturing(false));
    }
  };

  const captureFullscreen = async () => {
    dispatch(setError(null));
    dispatch(setPreview(null));
    dispatch(setMode('full'));
    dispatch(setCapturing(true));
    dispatch(setStep('capturing'));

    try {
      const img = await screenshotService.captureFullscreen();
      await handleCaptureResult(img);
    } catch (err: any) {
      dispatch(setError(err.message || 'Fullscreen capture failed'));
      dispatch(setCapturing(false));
    }
  };

  const captureRegion = async (x: number, y: number, w: number, h: number) => {
    dispatch(setError(null));
    dispatch(setPreview(null));
    dispatch(setMode('region'));
    dispatch(setCapturing(true));
    dispatch(setStep('capturing'));

    try {
      const img = await screenshotService.captureRegion(x, y, w, h);
      await handleCaptureResult(img);
    } catch (err: any) {
      dispatch(setError(err.message || 'Region capture failed'));
      dispatch(setCapturing(false));
    }
  };

  const startScrollCapture = async () => {
    dispatch(setError(null));
    dispatch(setPreview(null));
    dispatch(setMode('scroll'));
    dispatch(setCapturing(true));
    dispatch(setStep('capturing'));

    try {
      const img = await screenshotService.startScrollCapture();
      await handleCaptureResult(img);
    } catch (err: any) {
      dispatch(setError(err.message || 'Scroll capture failed'));
      dispatch(setCapturing(false));
    }
  };

  const openRegionSelector = async () => {
    try {
      await screenshotService.openRegionSelector();
    } catch (err: any) {
      dispatch(setError(err.message || 'Failed to open region selector window'));
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
