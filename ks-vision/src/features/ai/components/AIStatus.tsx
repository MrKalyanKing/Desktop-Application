import React, { useEffect } from 'react';
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
    <div className="lg-status">
      <span className="lg-status-dot" style={{ background: err ? 'var(--lg-danger)' : online ? 'var(--lg-success)' : 'var(--lg-text-muted)' }} />
      {online ? 'Ready' : err ? 'Error' : 'Offline'}
    </div>
  );
};
