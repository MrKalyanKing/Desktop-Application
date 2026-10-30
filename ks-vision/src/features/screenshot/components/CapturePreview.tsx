import React from 'react';
import { Image24Regular } from '@fluentui/react-icons';
import { useSelector } from 'react-redux';

export const CapturePreview: React.FC = () => {
  const preview = useSelector((state: any) => state.screenshot.currentPreview);

  if (!preview) return null;

  return (
    <div className="lg-card mb-2 flex gap-2 p-2 select-none">
      <div className="relative flex h-14 w-14 flex-shrink-0 items-center justify-center overflow-hidden rounded-[12px] border border-[var(--lg-rim)] bg-black/30">
        <img src={preview} alt="Capture" className="max-w-full max-h-full object-contain" />
      </div>
      <div className="flex-1 flex items-center gap-1.5 text-[11px] text-slate-300 font-medium">
        <Image24Regular className="text-[var(--lg-accent)]" style={{ fontSize: 16 }} />
        Screen captured
      </div>
    </div>
  );
};
