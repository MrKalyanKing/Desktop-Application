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
        'lg-btn',
        variant === 'primary' && 'lg-btn--primary',
        size === 'sm' && 'h-7 px-2.5 text-[11px]',
        size === 'lg' && 'h-11 px-6 text-base',
        className
      )}
      {...props}
    >
      {children}
    </button>
  );
};
