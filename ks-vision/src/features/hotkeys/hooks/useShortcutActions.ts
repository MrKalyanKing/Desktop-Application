import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

/** Action ids emitted by the Rust registry (modules/shortcuts). */
export type ShortcutActionId =
  | 'focus-input'
  | 'regenerate'
  | 'capture-full'
  | 'capture-window'
  | 'capture-region'
  | 'capture-scroll'
  | 'toggle-voice'
  | 'cycle-audio';

export interface ShortcutPayload {
  action: ShortcutActionId;
  targetHwnd?: number | null;
}

export interface RegionSelection {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ShortcutHandlers {
  onAction: (payload: ShortcutPayload) => void;
  onRegionSelected: (region: RegionSelection) => void;
  onCancelled: (ids: number[]) => void;
}

/**
 * The only frontend entry point for global shortcuts. Listeners are installed once;
 * handlers are read through a ref so they always see the latest component state.
 */
export const useShortcutActions = (handlers: ShortcutHandlers) => {
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  useEffect(() => {
    const unlisteners = [
      listen<ShortcutPayload>('ks://shortcut', (e) => {
        if (e.payload?.action) handlersRef.current.onAction(e.payload);
      }),
      listen<RegionSelection>('region-selected', (e) => {
        if (e.payload) handlersRef.current.onRegionSelected(e.payload);
      }),
      listen<{ ids: number[] }>('ks://cancelled', (e) => {
        handlersRef.current.onCancelled(e.payload?.ids ?? []);
      }),
    ];
    return () => {
      unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
    };
  }, []);
};
