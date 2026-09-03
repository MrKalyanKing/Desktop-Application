import { useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen, emit } from '@tauri-apps/api/event';
import { hotkeyService } from '../services/hotkey.service';
import { HOTKEYS, DOUBLE_PRESS_DELAY } from '../constants/hotkey.constants';
import { useScreenshot } from '../../screenshot';

export const useGlobalHotkeys = () => {
  const { captureFullscreen } = useScreenshot();

  useEffect(() => {
    let active = true;

    const setupGlobalShortcut = async () => {
      try {
        // Toggle widget visibility
        await hotkeyService.registerGlobal(HOTKEYS.TOGGLE_WIDGET, async (event) => {
          if (event.state === 'Pressed' && active) {
            const win = getCurrentWindow();
            const isVisible = await win.isVisible();
            if (isVisible) {
              await win.hide();
            } else {
              await win.show();
              await win.setFocus();
            }
          }
        });

        // Active Window capture: Ctrl+Shift+W (same capture path as F — no UI deadlock)
        await hotkeyService.registerGlobal('Ctrl+Shift+W', async (event) => {
          if (event.state === 'Pressed' && active) {
            captureFullscreen();
          }
        });

        // Full Screen capture: Ctrl+Shift+F
        await hotkeyService.registerGlobal('Ctrl+Shift+F', async (event) => {
          if (event.state === 'Pressed' && active) {
            captureFullscreen();
          }
        });

        // Region: same as fullscreen (region overlay window deadlocks this app)
        await hotkeyService.registerGlobal('Ctrl+Shift+R', async (event) => {
          if (event.state === 'Pressed' && active) {
            captureFullscreen();
          }
        });

        // Scroll: same as fullscreen (multi-page scroll froze the UI)
        await hotkeyService.registerGlobal('Ctrl+Shift+S', async (event) => {
          if (event.state === 'Pressed' && active) {
            captureFullscreen();
          }
        });

        // Voice record toggle: Ctrl+Shift+V
        await hotkeyService.registerGlobal('Ctrl+Shift+V', async (event) => {
          if (event.state === 'Pressed' && active) {
            await emit('toggle-voice');
          }
        });

      } catch (err) {
        console.error('Failed to setup global shortcut:', err);
      }
    };

    setupGlobalShortcut();

    // Listen for coordinates sent back from the transparent fullscreen region-selector window
    const regionListener = listen('region-selected', () => {
      if (active) captureFullscreen();
    });

    // Local double escape detection
    let lastEscapeTime = 0;
    const handleKeyDown = async (e: KeyboardEvent) => {
      if (e.key === HOTKEYS.LOCAL_ESCAPE) {
        const now = Date.now();
        if (now - lastEscapeTime < DOUBLE_PRESS_DELAY) {
          try {
            const win = getCurrentWindow();
            await win.hide();
          } catch (err) {
            console.error('Failed to hide window on Double Escape:', err);
          }
        }
        lastEscapeTime = now;
      }
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      active = false;
      window.removeEventListener('keydown', handleKeyDown);
      
      regionListener.then((unlisten) => unlisten());

      hotkeyService.unregisterGlobal(HOTKEYS.TOGGLE_WIDGET).catch((err) => {
        console.error('Failed to unregister global hotkey:', err);
      });
      hotkeyService.unregisterGlobal('Ctrl+Shift+W').catch(() => {});
      hotkeyService.unregisterGlobal('Ctrl+Shift+F').catch(() => {});
      hotkeyService.unregisterGlobal('Ctrl+Shift+R').catch(() => {});
      hotkeyService.unregisterGlobal('Ctrl+Shift+S').catch(() => {});
      hotkeyService.unregisterGlobal('Ctrl+Shift+V').catch(() => {});
    };
  }, []);
};
