import React from 'react';
import { cn } from '../../../shared/utils/cn';

interface WidgetIndicatorProps {
  active: boolean;
  className?: string;
}

export const WidgetIndicator: React.FC<WidgetIndicatorProps> = ({ active, className }) => {
  return (
    <span className={cn("relative flex h-2 w-2 select-none", className)}>
      {active && (
        <span className="pulse-indicator absolute inline-flex h-full w-full rounded-full bg-cyan-400 opacity-75" />
      )}
      <span className={cn(
        "relative inline-flex rounded-full h-2 w-2",
        active ? "bg-cyan-500" : "bg-slate-600"
      )} />
    </span>
  );
};
