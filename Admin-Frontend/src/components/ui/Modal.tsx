import type { ReactNode } from 'react';
import { cn } from '../../utils/cn';

export function Modal({
  open,
  title,
  children,
  onClose,
}: {
  open: boolean;
  title: string;
  children: ReactNode;
  onClose: () => void;
}) {
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 flex items-end justify-center p-4 sm:items-center">
      <button className="absolute inset-0 bg-black/50" onClick={onClose} aria-label="Close" />
      <div className={cn('glass-strong relative z-10 w-full max-w-lg rounded-[28px] p-6')}>
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-lg font-semibold">{title}</h2>
          <button onClick={onClose} className="glass-btn h-8 w-8 rounded-full">×</button>
        </div>
        {children}
      </div>
    </div>
  );
}
