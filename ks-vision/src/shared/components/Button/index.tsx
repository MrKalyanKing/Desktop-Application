import React from 'react';
import { cn } from '../../utils/cn';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
}

export const Button: React.FC<ButtonProps> = ({
  children,
  variant = 'primary',
  size = 'md',
  className,
  ...props
}) => {
  return (
    <button
      className={cn(
        "inline-flex items-center justify-center rounded-lg font-medium transition-all duration-200 focus:outline-none disabled:opacity-50 cursor-pointer active:scale-95",
        {
          'bg-cyan-600 hover:bg-cyan-500 text-white shadow-md shadow-cyan-900/25': variant === 'primary',
          'bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700': variant === 'secondary',
          'bg-transparent hover:bg-slate-800/60 text-slate-400 hover:text-slate-100': variant === 'ghost',
          'bg-rose-600 hover:bg-rose-500 text-white shadow-md shadow-rose-900/25': variant === 'danger',
          'h-7 px-2.5 text-xs rounded-md': size === 'sm',
          'h-9 px-4 py-2 text-sm': size === 'md',
          'h-11 px-6 text-base': size === 'lg',
        },
        className
      )}
      {...props}
    >
      {children}
    </button>
  );
};
