import { useDispatch, useSelector, useStore } from 'react-redux';
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
import { opsService, showNotice, type OpKind } from '../../hotkeys/services/ops.service';

function asErrorMessage(err: unknown, fallback: string): string {
  if (typeof err === 'string' && err.trim()) return err;
  if (err && typeof err === 'object') {
    const o = err as Record<string, unknown>;
    if (typeof o.message === 'string' && o.message.trim()) return o.message;
  }
  return fallback;
}

type CaptureMode = 'active' | 'full' | 'region' | 'scroll';

const TIMEOUT_MS: Record<CaptureMode, number> = {
  full: 10000,
  active: 10000,
  region: 10000,
  scroll: 45000,
};

/** Op id of the capture currently running (one at a time). */
let activeCaptureOp: number | null = null;

export const useScreenshot = () => {
  const dispatch = useDispatch();
  const store = useStore();
  const isCapturing = useSelector((state: any) => state.screenshot.isCapturing);
  const isProcessing = useSelector((state: any) => state.screenshot.isProcessing);
  const step = useSelector((state: any) => state.screenshot.step);
  const currentPreview = useSelector((state: any) => state.screenshot.currentPreview);
  const lastOcrResult = useSelector((state: any) => state.screenshot.lastOcrResult);
  const error = useSelector((state: any) => state.screenshot.error);
  const mode = useSelector((state: any) => state.screenshot.mode);

  const { presentVoiceAnswer } = useAI();

  /** Read live store state: closures from hotkey listeners would otherwise be stale. */
  const isBusy = () => {
    const s = (store.getState() as any).screenshot;
    return s.isCapturing || s.isProcessing;
  };

  const runCapture = async (
    mode: CaptureMode,
    fn: (opId: number) => Promise<string>
  ) => {
    if (isBusy()) return;
    dispatch(setCapturing(true));

    const kind: OpKind = mode === 'scroll' ? 'scroll' : 'capture';
    let opId: number;
    try {
      opId = await opsService.begin(kind);
    } catch (err) {
      dispatch(setCapturing(false));
      dispatch(setError(asErrorMessage(err, 'Screenshot capture failed')));
      return;
    }
    activeCaptureOp = opId;

    dispatch(setError(null));
    dispatch(setPreview(null));
    dispatch(setOcrResult(null));
    dispatch(setMode(mode));
    dispatch(setStep('capturing'));

    try {
      const img = await Promise.race([
        fn(opId),
        new Promise<string>((_, reject) =>
          setTimeout(() => reject(new Error('Screenshot timed out')), TIMEOUT_MS[mode])
        ),
      ]);
      if (opsService.isCancelled(opId)) return;

      dispatch(setPreview(img));
      dispatch(setCapturing(false));
      dispatch(setProcessing(true));
      dispatch(setStep('ai'));

      const answer = await screenshotService.analyze(img, opId);
      if (opsService.isCancelled(opId)) return;
      if (answer?.trim()) {
        await presentVoiceAnswer(answer.trim(), 'OCR', opId);
      }
      dispatch(setStep('done'));
    } catch (err: unknown) {
      if (opsService.isCancelled(opId)) return;
      const msg = asErrorMessage(err, 'Screenshot capture failed');
      if (msg === 'cancelled') {
        // The invoke can settle before the ks://cancelled broadcast arrives.
        dispatch(setStep('idle'));
        return;
      }
      console.error('Screenshot failed:', err);
      dispatch(setError(msg));
    } finally {
      opsService.end(opId);
      if (activeCaptureOp === opId) {
        activeCaptureOp = null;
        dispatch(setCapturing(false));
        dispatch(setProcessing(false));
        if (opsService.isCancelled(opId)) dispatch(setStep('idle'));
      }
    }
  };

  const captureActiveWindow = (targetHwnd?: number | null) =>
    runCapture('active', () => screenshotService.captureActiveWindow(targetHwnd));
  const captureFullscreen = () => runCapture('full', screenshotService.captureFullscreen);
  const captureRegion = (x: number, y: number, w: number, h: number) =>
    runCapture('region', () => screenshotService.captureRegion(x, y, w, h));
  const startScrollCapture = (targetHwnd: number) =>
    runCapture('scroll', async (opId) => {
      const result = await screenshotService.startScrollCapture(targetHwnd, opId);
      showNotice(result.status === 'partial' ? 'error' : 'info', result.reason);
      return result.image;
    });

  const openRegionSelector = async () => {
    if (isBusy()) return;
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
