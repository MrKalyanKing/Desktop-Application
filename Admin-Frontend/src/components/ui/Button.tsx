import type { ButtonHTMLAttributes, ReactNode } from 'react';
import { cn } from '../../utils/cn';

export function Button({
  children,
  className,
  variant = 'default',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  children: ReactNode;
  variant?: 'default' | 'primary' | 'danger';
}) {
  return (
    <button
      className={cn(
        'glass-btn inline-flex items-center justify-center gap-2 rounded-full px-4 py-2 text-sm font-medium',
        variant === 'primary' && 'glass-btn-primary',
        variant === 'danger' && 'border-rose-400/40 text-rose-200',
        className,
      )}
      {...props}
    >
      {children}
    </button>
  );
}
