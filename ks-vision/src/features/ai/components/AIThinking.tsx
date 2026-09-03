import React from 'react';

export const AIThinking: React.FC = () => {
  return (
    <div className="flex items-center gap-2 px-2 py-1 select-none">
      <span className="text-[11px] text-slate-400 font-medium">Thinking</span>
      <span className="flex gap-1 items-center">
        <span className="h-1.5 w-1.5 bg-cyan-400 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
        <span className="h-1.5 w-1.5 bg-violet-400 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
        <span className="h-1.5 w-1.5 bg-sky-400 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
      </span>
    </div>
  );
};
