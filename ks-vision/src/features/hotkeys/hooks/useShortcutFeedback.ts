import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { NoticeLevel } from '../services/ops.service';

export interface Notice {
  level: NoticeLevel;
  message: string;
}

interface KeyStatus {
  key: string;
  action: string;
  registered: boolean;
  error: string | null;
}

const NOTICE_MS = 3000;
const DEDUPE_MS = 2000;

/**
 * One small status line (`ks://notice`, same text dropped within 2s, fades after 3s)
 * plus persistent badges for shortcuts the OS refused to register.
 */
export const useShortcutFeedback = () => {
  const [notice, setNotice] = useState<Notice | null>(null);
  const [failedKeys, setFailedKeys] = useState<{ key: string; error: string }[]>([]);
  const lastRef = useRef<{ message: string; at: number }>({ message: '', at: 0 });
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const failures = new Map<string, string>();
    const apply = (s: KeyStatus) => {
      if (s.error) failures.set(s.key, s.error);
      else if (s.registered) failures.delete(s.key);
      setFailedKeys(Array.from(failures, ([key, error]) => ({ key, error })));
    };

    invoke<KeyStatus[]>('get_shortcut_status_cmd')
      .then((list) => list.forEach(apply))
      .catch(() => {});

    const unlisteners = [
      listen<KeyStatus>('ks://shortcut-status', (e) => e.payload && apply(e.payload)),
      listen<Notice>('ks://notice', (e) => {
        const n = e.payload;
        if (!n?.message) return;
        const now = Date.now();
        if (lastRef.current.message === n.message && now - lastRef.current.at < DEDUPE_MS) return;
        lastRef.current = { message: n.message, at: now };
        setNotice(n);
        if (timerRef.current) clearTimeout(timerRef.current);
        timerRef.current = setTimeout(() => setNotice(null), NOTICE_MS);
      }),
    ];

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
      unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
    };
  }, []);

  return { notice, failedKeys };
};
