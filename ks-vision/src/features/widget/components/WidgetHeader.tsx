import React from 'react';
import { Dismiss24Regular, Sparkle24Filled } from '@fluentui/react-icons';
import { Button } from '../../../shared/components/Button';
import { widgetService } from '../services/widget.service';

export const WidgetHeader: React.FC = () => {
  const handleClose = async () => {
    await widgetService.hide();
  };

  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between h-11 px-3 border-b border-white/5 cursor-move select-none shrink-0"
      style={{ background: 'linear-gradient(90deg, rgba(34,211,238,0.08), rgba(167,139,250,0.08))' }}
    >
      <div data-tauri-drag-region className="flex items-center gap-2 cursor-move min-w-0">
        <div className="h-7 w-7 rounded-lg flex items-center justify-center bg-cyan-400/15 text-cyan-300 ring-1 ring-cyan-400/25">
          <Sparkle24Filled style={{ fontSize: 16 }} />
        </div>
        <div className="flex flex-col leading-tight min-w-0">
          <span
            data-tauri-drag-region
            className="text-[12px] font-semibold tracking-wide text-transparent bg-clip-text bg-gradient-to-r from-cyan-300 via-sky-300 to-violet-300"
          >
            KS Vision
          </span>
          <span data-tauri-drag-region className="text-[9px] text-slate-500">
            Drag to move
          </span>
        </div>
      </div>

      <Button
        variant="ghost"
        size="sm"
        onClick={handleClose}
        className="h-7 w-7 p-0 rounded-lg hover:bg-rose-500/20 hover:text-rose-300 text-slate-400"
        title="Hide overlay (double Escape)"
      >
        <Dismiss24Regular style={{ fontSize: 16 }} />
      </Button>
    </div>
  );
};
