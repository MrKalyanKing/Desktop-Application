import React, { useEffect, useRef } from 'react';
import { useRegionSelection } from '../hooks/useRegionSelection';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { emit } from '@tauri-apps/api/event';

export const RegionSelector: React.FC = () => {
  const {
    isSelecting,
    selectionRect,
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
    handleArrowKey,
  } = useRegionSelection();
  const doneRef = useRef(false);
  const rectRef = useRef(selectionRect);
  rectRef.current = selectionRect;

  const close = async () => {
    try {
      await getCurrentWindow().close();
    } catch (err) {
      console.error('Failed to close region selector:', err);
    }
  };

  const cancel = async () => {
    if (doneRef.current) return;
    doneRef.current = true;
    await emit('ks://notice', { level: 'info', message: 'Region cancelled' });
    await close();
  };

  /** Convert the CSS-pixel selection to physical screen pixels and hand it off. */
  const confirm = async () => {
    const r = rectRef.current;
    if (doneRef.current || r.width < 5 || r.height < 5) return;
    doneRef.current = true;
    const win = getCurrentWindow();
    const origin = await win.outerPosition();
    const dpr = window.devicePixelRatio || 1;
    await emit('region-selected', {
      x: origin.x + Math.round(r.x * dpr),
      y: origin.y + Math.round(r.y * dpr),
      width: Math.round(r.width * dpr),
      height: Math.round(r.height * dpr),
    });
    await close();
  };

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const showWindow = async () => {
      try {
        const win = getCurrentWindow();
        await win.show();
        await win.setFocus();

        // OS Safety Failsafe: close the window if it loses focus (blur)
        unlisten = await win.onFocusChanged(({ payload: focused }) => {
          if (!focused) cancel();
        });
      } catch (err) {
        console.error('Failed to show region selector window:', err);
      }
    };

    // Tiny delay to ensure styles and transparency are fully applied by WebView2
    const timer = setTimeout(showWindow, 100);
    return () => {
      clearTimeout(timer);
      if (unlisten) {
        unlisten();
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        cancel();
      } else if (e.key === 'Enter') {
        e.preventDefault();
        confirm();
      } else if (handleArrowKey(e)) {
        e.preventDefault();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleMouseUpAndConfirm = async () => {
    const wasDragging = isSelecting;
    handleMouseUp();
    if (wasDragging && selectionRect.width > 10 && selectionRect.height > 10) {
      await confirm();
    }
  };

  return (
    <div
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUpAndConfirm}
      className="relative w-screen h-screen bg-slate-950/60 cursor-crosshair select-none overflow-hidden"
    >
      <div className="absolute top-8 left-1/2 -translate-x-1/2 px-4 py-2 bg-slate-900/90 backdrop-blur border border-slate-800 rounded-lg text-xs text-slate-200 font-medium pointer-events-none shadow-2xl flex items-center gap-2">
        <span>📸</span>
        <span>
          <strong>Arrows</strong> move · <strong>Shift+Arrows</strong> resize · <strong>Ctrl</strong> fine step ·{' '}
          <strong>Enter</strong> capture · <strong>Esc</strong> cancel · or drag with the mouse
        </span>
      </div>

      <div
        className="absolute border border-cyan-400 bg-cyan-400/5 shadow-[0_0_15px_rgba(34,211,238,0.2)] pointer-events-none"
        style={{
          left: `${selectionRect.x}px`,
          top: `${selectionRect.y}px`,
          width: `${selectionRect.width}px`,
          height: `${selectionRect.height}px`,
        }}
      >
        <div className="absolute -top-6 left-0 bg-cyan-950 border border-cyan-800 px-1.5 py-0.5 rounded text-[9px] font-mono text-cyan-300">
          {Math.round(selectionRect.width * (window.devicePixelRatio || 1))} x{' '}
          {Math.round(selectionRect.height * (window.devicePixelRatio || 1))}
        </div>
      </div>
    </div>
  );
};
