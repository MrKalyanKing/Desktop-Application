import React, { useEffect } from 'react';
import { CheckmarkCircle24Filled, ErrorCircle24Filled, Circle24Regular } from '@fluentui/react-icons';
import { useAI } from '../hooks/useAI';

export const AIStatus: React.FC = () => {
  const { status, healthCheck } = useAI();

  useEffect(() => {
    healthCheck();
    const interval = setInterval(healthCheck, 10000);
    return () => clearInterval(interval);
  }, []);

  const online = status === 'connected';
  const err = status === 'error';

  return (
    <div
      className={`flex items-center gap-0.5 px-1.5 py-0 rounded-full text-[8px] font-semibold select-none border leading-tight ${
        online
          ? 'bg-emerald-500/10 border-emerald-400/25 text-emerald-300'
          : err
            ? 'bg-rose-500/10 border-rose-400/25 text-rose-300'
            : 'bg-white/5 border-white/10 text-slate-400'
      }`}
    >
      {online ? (
        <CheckmarkCircle24Filled style={{ fontSize: 10 }} />
      ) : err ? (
        <ErrorCircle24Filled style={{ fontSize: 10 }} />
      ) : (
        <Circle24Regular style={{ fontSize: 10 }} />
      )}
      {online ? 'Ready' : err ? 'Error' : 'Offline'}
    </div>
  );
};
