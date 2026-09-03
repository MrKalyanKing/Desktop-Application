import React from 'react';
import { cn } from '../../../shared/utils/cn';

interface WidgetContainerProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
  opacity: number;
}

export const WidgetContainer: React.FC<WidgetContainerProps> = ({ 
  children, 
  opacity, 
  className, 
  ...props 
}) => {
  return (
    <div
      className={cn(
        "widget-glass relative w-full h-full rounded-2xl overflow-hidden flex flex-col select-none",
        className
      )}
      style={{ opacity, transition: 'opacity 0.25s cubic-bezier(0.4, 0, 0.2, 1)' }}
      {...props}
    >
      {children}
    </div>
  );
};
