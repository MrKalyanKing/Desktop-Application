import React from 'react';
import { Dismiss24Regular, Sparkle24Filled } from '@fluentui/react-icons';
import { widgetService } from '../services/widget.service';

export const WidgetHeader: React.FC = () => {
  const handleClose = async () => {
    await widgetService.hide();
  };

  return (
    <div data-tauri-drag-region className="lg-header">
      <div data-tauri-drag-region className="flex min-w-0 cursor-move items-center gap-2.5">
        <div className="lg-roundel">
          <Sparkle24Filled style={{ fontSize: 16 }} />
        </div>
        <div className="flex min-w-0 flex-col leading-tight">
          <span data-tauri-drag-region className="text-[15px] font-semibold tracking-[-0.02em] text-white">
            KS Vision
          </span>
          <span data-tauri-drag-region className="text-[11px] tracking-wide text-[var(--lg-text-muted)]">
            Drag to move
          </span>
        </div>
      </div>

      <button
        type="button"
        onClick={handleClose}
        className="lg-icon-btn"
        title="Hide overlay (double Escape)"
      >
        <Dismiss24Regular style={{ fontSize: 16 }} />
      </button>
    </div>
  );
};
