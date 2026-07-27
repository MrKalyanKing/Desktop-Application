import React from 'react';
import { cn } from '../../utils/cn';

interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
}

export const Card: React.FC<CardProps> = ({ children, className, ...props }) => {
  return (
    <div 
      className={cn(
        "rounded-xl border bg-slate-900/40 text-slate-100 shadow-xl backdrop-blur-md border-slate-700/30", 
        className
      )} 
      {...props}
    >
      {children}
    </div>
  );
};
