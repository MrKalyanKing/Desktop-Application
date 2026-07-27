import { createSlice } from '@reduxjs/toolkit';
import type { PayloadAction } from '@reduxjs/toolkit';
import type { ScreenshotState, ScreenshotStep, OcrResult, ScreenshotMode } from '../types/screenshot.types';

const initialState: ScreenshotState = {
  isCapturing: false,
  isProcessing: false,
  step: 'idle',
  currentPreview: null,
  lastOcrResult: null,
  error: null,
  mode: null,
};

export const screenshotSlice = createSlice({
  name: 'screenshot',
  initialState,
  reducers: {
    setCapturing: (state, action: PayloadAction<boolean>) => {
      state.isCapturing = action.payload;
    },
    setProcessing: (state, action: PayloadAction<boolean>) => {
      state.isProcessing = action.payload;
    },
    setStep: (state, action: PayloadAction<ScreenshotStep>) => {
      state.step = action.payload;
    },
    setPreview: (state, action: PayloadAction<string | null>) => {
      state.currentPreview = action.payload;
    },
    setOcrResult: (state, action: PayloadAction<OcrResult | null>) => {
      state.lastOcrResult = action.payload;
    },
    setError: (state, action: PayloadAction<string | null>) => {
      state.error = action.payload;
      state.step = action.payload ? 'error' : 'idle';
    },
    setMode: (state, action: PayloadAction<ScreenshotMode | null>) => {
      state.mode = action.payload;
    },
    resetScreenshotState: (state) => {
      Object.assign(state, initialState);
    },
  },
});

export const {
  setCapturing,
  setProcessing,
  setStep,
  setPreview,
  setOcrResult,
  setError,
  setMode,
  resetScreenshotState,
} = screenshotSlice.actions;

export default screenshotSlice.reducer;
