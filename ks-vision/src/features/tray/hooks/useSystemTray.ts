import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';

interface UseSystemTrayProps {
  onOpenSettings: () => void;
  onCheckStatus: () => void;
  onRestartConnection: () => void;
}

export const useSystemTray = ({
  onOpenSettings,
  onCheckStatus,
  onRestartConnection,
}: UseSystemTrayProps) => {
  useEffect(() => {
    let unlistenOpenSettings: (() => void) | undefined;
    let unlistenCheckStatus: (() => void) | undefined;
    let unlistenRestart: (() => void) | undefined;

    const setupListeners = async () => {
      unlistenOpenSettings = await listen('open-settings', () => {
        onOpenSettings();
      });

      unlistenCheckStatus = await listen('check-ai-status', () => {
        onCheckStatus();
      });

      unlistenRestart = await listen('restart-gemini', () => {
        onRestartConnection();
      });
    };

    setupListeners();

    return () => {
      if (unlistenOpenSettings) unlistenOpenSettings();
      if (unlistenCheckStatus) unlistenCheckStatus();
      if (unlistenRestart) unlistenRestart();
    };
  }, [onOpenSettings, onCheckStatus, onRestartConnection]);
};
