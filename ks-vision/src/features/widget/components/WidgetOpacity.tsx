import React from 'react';

interface WidgetOpacityProps {
  opacity: number;
}

export const WidgetOpacity: React.FC<WidgetOpacityProps> = ({ opacity }) => {
  return (
    <div className="absolute bottom-1 right-2 text-[8px] font-mono text-slate-500 bg-slate-950/20 px-1 py-0.5 rounded border border-slate-800/10 select-none">
      O: {(opacity * 100).toFixed(0)}%
    </div>
  );
};
