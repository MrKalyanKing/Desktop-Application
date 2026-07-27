import React, { useState } from 'react';
import { cn } from '../../utils/cn';

interface TooltipProps {
  content: string;
  children: React.ReactElement;
  className?: string;
}

export const Tooltip: React.FC<TooltipProps> = ({ content, children, className }) => {
  const [visible, setVisible] = useState(false);

  return (
    <div
      className="relative inline-block"
      onMouseEnter={() => setVisible(true)}
      onMouseLeave={() => setVisible(false)}
    >
      {children}
      {visible && (
        <div 
          className={cn(
            "absolute bottom-full left-1/2 -translate-x-1/2 mb-1.5 px-2 py-1 text-[10px] font-medium text-slate-200 bg-slate-950 border border-slate-800 rounded shadow-premium backdrop-blur-sm whitespace-nowrap z-50 animate-fade-in",
            className
          )}
        >
          {content}
        </div>
      )}
    </div>
  );
};
