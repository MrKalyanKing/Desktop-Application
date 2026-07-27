import React from 'react';
import { DragIcon, CloseIcon } from '../../../shared/components/Icon';
import { Button } from '../../../shared/components/Button';
import { widgetService } from '../services/widget.service';

export const WidgetHeader: React.FC = () => {
  const handleClose = async () => {
    await widgetService.hide();
  };

  return (
    <div
      data-tauri-drag-region
      className="flex items-center justify-between h-9 px-3 bg-slate-950/40 border-b border-slate-800/30 cursor-move text-slate-400 select-none"
    >
      <div data-tauri-drag-region className="flex items-center gap-1.5 cursor-move">
        <DragIcon size={12} className="text-slate-500 cursor-move" />
        <span 
          data-tauri-drag-region 
          className="text-[10px] font-bold tracking-widest text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-purple-400 uppercase cursor-move"
        >
          KS Vision
        </span>
      </div>
      
      <Button
        variant="ghost"
        size="sm"
        onClick={handleClose}
        className="h-5 w-5 p-0 hover:bg-slate-800/80 hover:text-slate-100 rounded-md transition-all duration-150 cursor-pointer"
        title="Hide Widget (Double Escape)"
      >
        <CloseIcon size={11} />
      </Button>
    </div>
  );
};
