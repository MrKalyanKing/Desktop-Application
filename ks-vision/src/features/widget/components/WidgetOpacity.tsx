import React from 'react';

interface WidgetOpacityProps {
  opacity: number;
}

export const WidgetOpacity: React.FC<WidgetOpacityProps> = ({ opacity }) => {
  return (
    <div className="absolute bottom-1.5 right-2 text-[9px] font-medium text-slate-500/70 select-none pointer-events-none z-0">
      {Math.round(opacity * 100)}%
    </div>
  );
};
