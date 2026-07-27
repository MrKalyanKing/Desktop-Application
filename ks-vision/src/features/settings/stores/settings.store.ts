import { createSlice } from '@reduxjs/toolkit';
import type { PayloadAction } from '@reduxjs/toolkit';
import type { AppSettings } from '../types/settings.types';

interface SettingsState {
  preferences: AppSettings | null;
  loading: boolean;
  error: string | null;
}

const initialState: SettingsState = {
  preferences: null,
  loading: false,
  error: null,
};

export const settingsSlice = createSlice({
  name: 'settings',
  initialState,
  reducers: {
    setLoading: (state, action: PayloadAction<boolean>) => {
      state.loading = action.payload;
    },
    setPreferences: (state, action: PayloadAction<AppSettings>) => {
      state.preferences = action.payload;
      state.error = null;
    },
    setError: (state, action: PayloadAction<string | null>) => {
      state.error = action.payload;
    },
  },
});

export const { setLoading, setPreferences, setError } = settingsSlice.actions;
export default settingsSlice.reducer;
