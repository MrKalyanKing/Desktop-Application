import React from 'react';

export const AIThinking: React.FC = () => {
  return (
    <div className="flex items-center gap-2 px-2 py-1 select-none">
      <span className="text-[11px] font-medium text-[var(--lg-text-secondary)]">Thinking</span>
      <span className="flex items-center gap-1">
        <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-[var(--lg-accent)]" style={{ animationDelay: '0ms' }} />
        <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-[var(--lg-accent-deep)]" style={{ animationDelay: '150ms' }} />
        <span className="h-1.5 w-1.5 animate-bounce rounded-full bg-[var(--lg-text-muted)]" style={{ animationDelay: '300ms' }} />
      </span>
    </div>
  );
};
