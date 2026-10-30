import { useEffect, useRef } from 'react';
import { useStore } from 'react-redux';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { widgetService } from '../services/widget.service';
import { AUTO_HIDE_MIN_MS } from '../constants/widget.constants';

/**
 * Auto-hide driven by `widget.autoHideDuration` (seconds; 0 = disabled, the default).
 * The effective idle delay is at least 60s so a reader is never cut off. Never hides
 * while generating, streaming, capturing, recording, or with focus in the overlay.
 */
export const useAutoHide = (isHovered: boolean) => {
  const store = useStore();
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const delayRef = useRef(0);
  const hoveredRef = useRef(isHovered);
  hoveredRef.current = isHovered;

  const clear = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  };

  const isBusy = async () => {
    const s = store.getState() as any;
    if (s.ai.loading || s.ai.streaming) return true;
    if (s.screenshot.isCapturing || s.screenshot.isProcessing) return true;
    if (hoveredRef.current || document.hasFocus()) return true;
    try {
      const audio = await invoke<{ is_recording: boolean }>('get_audio_capture_state');
      return audio.is_recording;
    } catch {
      return true;
    }
  };

  const schedule = () => {
    clear();
    if (delayRef.current <= 0) return;
    timerRef.current = setTimeout(async () => {
      if (await isBusy()) {
        schedule();
        return;
      }
      await widgetService.hide();
    }, delayRef.current);
  };

  useEffect(() => {
    let alive = true;
    invoke<string>('load_settings_cmd')
      .then((raw) => {
        if (!alive) return;
        const seconds = Number(JSON.parse(raw)?.widget?.autoHideDuration) || 0;
        delayRef.current = seconds > 0 ? Math.max(seconds * 1000, AUTO_HIDE_MIN_MS) : 0;
        schedule();
      })
      .catch(() => {
        delayRef.current = 0;
      });

    const onActivity = () => schedule();
    window.addEventListener('mousemove', onActivity);
    window.addEventListener('mousedown', onActivity);
    window.addEventListener('keydown', onActivity);

    // Shortcuts and completed answers count as activity.
    const unlisteners = [
      listen('ks://shortcut', onActivity),
      listen('ks://visibility', onActivity),
      listen('voice-gemini-answer', onActivity),
    ];
    const unsubscribe = store.subscribe(() => {
      const s = store.getState() as any;
      if (s.ai.loading || s.ai.streaming) schedule();
    });

    return () => {
      alive = false;
      clear();
      unsubscribe();
      window.removeEventListener('mousemove', onActivity);
      window.removeEventListener('mousedown', onActivity);
      window.removeEventListener('keydown', onActivity);
      unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!isHovered) schedule();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isHovered]);
};
