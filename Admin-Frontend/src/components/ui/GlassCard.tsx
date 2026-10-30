import type { HTMLAttributes, ReactNode } from 'react';
import { cn } from '../../utils/cn';

export function GlassCard({
  children,
  className,
  strong = false,
  ...props
}: HTMLAttributes<HTMLDivElement> & {
  children: ReactNode;
  strong?: boolean;
}) {
  return (
    <div className={cn(strong ? 'glass-strong' : 'glass', 'rounded-[24px]', className)} {...props}>
      {children}
    </div>
  );
}
