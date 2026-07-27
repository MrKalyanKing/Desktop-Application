import React, { useEffect } from 'react';
import { useAI } from '../hooks/useAI';

export const AIStatus: React.FC = () => {
  const { status, healthCheck } = useAI();

  useEffect(() => {
    healthCheck();
    const interval = setInterval(healthCheck, 10000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-slate-950/20 border border-slate-800/30 text-[9px] font-bold tracking-wide select-none">
      <span className="relative flex h-1.5 w-1.5">
        {status === 'connected' && (
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" />
        )}
        <span className={`relative inline-flex rounded-full h-1.5 w-1.5 ${
          status === 'connected' ? 'bg-emerald-500' :
          status === 'error' ? 'bg-rose-500' : 'bg-slate-500'
        }`} />
      </span>
      <span className={
        status === 'connected' ? 'text-emerald-400/90' :
        status === 'error' ? 'text-rose-400/90' : 'text-slate-500'
      }>
        {status === 'connected' ? 'OLLAMA ONLINE' :
         status === 'error' ? 'OLLAMA ERROR' : 'OLLAMA OFFLINE'}
      </span>
    </div>
  );
};
