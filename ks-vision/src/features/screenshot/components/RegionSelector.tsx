import React, { useEffect } from 'react';
import { useRegionSelection } from '../hooks/useRegionSelection';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { emit } from '@tauri-apps/api/event';

export const RegionSelector: React.FC = () => {
  const {
    selectionRect,
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
  } = useRegionSelection();

  const handleConfirm = async () => {
    if (!selectionRect) return;
    const { x, y, width, height } = selectionRect;
    if (width < 5 || height < 5) return;

    await emit('region-selected', { x, y, width, height });

    const win = getCurrentWindow();
    await win.close();
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
          if (!focused) {
            win.close();
          }
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
  }, []);

  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        const win = getCurrentWindow();
        await win.close();
      } else if (e.key === 'Enter') {
        await handleConfirm();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [selectionRect]);

  const handleMouseUpAndConfirm = async () => {
    handleMouseUp();
    if (selectionRect && selectionRect.width > 10 && selectionRect.height > 10) {
      await handleConfirm();
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
        <span>Drag a rectangle to capture | Press <strong>Esc</strong> to cancel</span>
      </div>

      {selectionRect && (
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
            {selectionRect.width} x {selectionRect.height}
          </div>
        </div>
      )}
    </div>
  );
};
