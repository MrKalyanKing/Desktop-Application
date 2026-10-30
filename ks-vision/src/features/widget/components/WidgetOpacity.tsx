import React from 'react';

interface WidgetOpacityProps {
  opacity: number;
}

export const WidgetOpacity: React.FC<WidgetOpacityProps> = ({ opacity }) => {
  return (
    <div className="pointer-events-none absolute bottom-2 right-4 z-10 select-none text-[9px] font-medium text-[var(--lg-text-muted)]">
      {Math.round(opacity * 100)}%
    </div>
  );
};
