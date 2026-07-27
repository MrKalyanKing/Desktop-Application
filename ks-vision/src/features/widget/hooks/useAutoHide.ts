import { useEffect, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { AUTO_HIDE_DELAY } from '../constants/widget.constants';

export const useAutoHide = (isHovered: boolean, visible: boolean, delay = AUTO_HIDE_DELAY) => {
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const resetTimer = () => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    if (visible && !isHovered) {
      timerRef.current = setTimeout(async () => {
        try {
          const win = getCurrentWindow();
          await win.hide();
        } catch (e) {
          console.error('Failed to auto-hide window:', e);
        }
      }, delay);
    }
  };

  useEffect(() => {
    resetTimer();
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [isHovered, visible, delay]);

  useEffect(() => {
    const handleActivity = () => {
      resetTimer();
    };

    window.addEventListener('mousemove', handleActivity);
    window.addEventListener('mousedown', handleActivity);
    window.addEventListener('keydown', handleActivity);

    return () => {
      window.removeEventListener('mousemove', handleActivity);
      window.removeEventListener('mousedown', handleActivity);
      window.removeEventListener('keydown', handleActivity);
    };
  }, [isHovered, visible, delay]);
};
