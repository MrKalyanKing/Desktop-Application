import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

/**
 * OS cursor is hidden while the pointer is over this window (so screen-share
 * tools do not draw a system cursor). A custom cursor is painted inside the
 * capture-excluded webview so only the local user (admin) can see it.
 */
export const LocalOnlyCursor: React.FC = () => {
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  const [active, setActive] = useState(false);

  useEffect(() => {
    const setOsCursor = async (visible: boolean) => {
      try {
        await invoke('set_os_cursor_visible', { visible });
      } catch {
        // non-Tauri / missing command — CSS cursor:none still applies
      }
    };

    const onMove = (e: MouseEvent) => {
      setPos({ x: e.clientX, y: e.clientY });
    };

    const onEnter = () => {
      setActive(true);
      void setOsCursor(false);
    };

    const onLeave = () => {
      setActive(false);
      setPos(null);
      void setOsCursor(true);
    };

    void setOsCursor(false);
    setActive(true);

    window.addEventListener('mousemove', onMove);
    document.addEventListener('mouseenter', onEnter);
    document.addEventListener('mouseleave', onLeave);

    return () => {
      window.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseenter', onEnter);
      document.removeEventListener('mouseleave', onLeave);
      void setOsCursor(true);
    };
  }, []);

  if (!active || !pos) return null;

  return (
    <div
      aria-hidden
      className="local-only-cursor"
      style={{
        transform: `translate(${pos.x}px, ${pos.y}px)`,
      }}
    >
      <svg width="16" height="20" viewBox="0 0 16 20" fill="none">
        <path
          d="M1 1L1 17L5.2 13.5L8.5 19L11 17.5L7.8 12H14L1 1Z"
          fill="#ffffff"
          stroke="#111827"
          strokeWidth="1.2"
          strokeLinejoin="round"
        />
      </svg>
    </div>
  );
};
