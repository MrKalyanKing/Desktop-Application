import React from 'react';

export const AIThinking: React.FC = () => {
  return (
    <div className="flex items-center gap-1.5 px-1 py-0.5 select-none">
      <span className="text-[9px] text-slate-500 font-bold tracking-widest">AI THINKING</span>
      <span className="flex gap-1 items-center">
        <span className="h-1 w-1 bg-cyan-400 rounded-full animate-bounce" style={{ animationDelay: '0ms' }} />
        <span className="h-1 w-1 bg-cyan-400 rounded-full animate-bounce" style={{ animationDelay: '150ms' }} />
        <span className="h-1 w-1 bg-cyan-400 rounded-full animate-bounce" style={{ animationDelay: '300ms' }} />
      </span>
    </div>
  );
};
