import React from 'react';
import { cn } from '../../utils/cn';

interface LoaderProps {
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export const Loader: React.FC<LoaderProps> = ({ size = 'md', className }) => {
  return (
    <div
      className={cn(
        "animate-spin rounded-full border-2 border-slate-700/50 border-t-cyan-500",
        {
          "h-4 w-4": size === 'sm',
          "h-7 w-7": size === 'md',
          "h-10 w-10": size === 'lg',
        },
        className
      )}
    />
  );
};
