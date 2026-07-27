import { useEffect } from 'react';
import { useDispatch, useSelector } from 'react-redux';
import { setPreferences, setLoading, setError } from '../stores/settings.store';
import { settingsService } from '../services/settings.service';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
import type { AppSettings } from '../types/settings.types';

export const useSettings = () => {
  const dispatch = useDispatch();
  const preferences = useSelector((state: any) => state.settings.preferences);
  const loading = useSelector((state: any) => state.settings.loading);
  const error = useSelector((state: any) => state.settings.error);

  const fetchSettings = async () => {
    dispatch(setLoading(true));
    try {
      const data = await settingsService.loadSettings();
      const isStartup = await settingsService.isStartupEnabled();
      const merged: AppSettings = { ...data, launchOnStartup: isStartup };
      dispatch(setPreferences(merged));
      await applyWindowSettings(merged);
    } catch (err: any) {
      dispatch(setError(err.message || 'Failed to load preferences'));
    } finally {
      dispatch(setLoading(false));
    }
  };

  const saveSettings = async (newSettings: AppSettings) => {
    dispatch(setLoading(true));
    try {
      await settingsService.saveSettings(newSettings);
      dispatch(setPreferences(newSettings));
      await applyWindowSettings(newSettings);
    } catch (err: any) {
      dispatch(setError(err.message || 'Failed to save preferences'));
    } finally {
      dispatch(setLoading(false));
    }
  };

  const applyWindowSettings = async (settings: AppSettings) => {
    try {
      const win = getCurrentWindow();
      await win.setAlwaysOnTop(settings.widget.alwaysOnTop);
      const width = settings.widget.width || 300;
      const height = settings.widget.height || 150;
      await win.setSize(new LogicalSize(width, height));
    } catch (e) {
      console.warn('Failed to apply window configurations dynamically:', e);
    }
  };

  useEffect(() => {
    if (!preferences) {
      fetchSettings();
    }
  }, []);

  return {
    preferences,
    loading,
    error,
    saveSettings,
    refreshSettings: fetchSettings,
  };
};
