import React from 'react';
import { Image24Regular } from '@fluentui/react-icons';
import { useSelector } from 'react-redux';

export const CapturePreview: React.FC = () => {
  const preview = useSelector((state: any) => state.screenshot.currentPreview);

  if (!preview) return null;

  return (
    <div className="flex gap-2 bg-white/4 p-2 rounded-xl border border-white/8 select-none mb-2">
      <div className="relative w-14 h-14 bg-black/40 border border-white/10 rounded-lg overflow-hidden flex-shrink-0 flex items-center justify-center">
        <img src={preview} alt="Capture" className="max-w-full max-h-full object-contain" />
      </div>
      <div className="flex-1 flex items-center gap-1.5 text-[11px] text-slate-300 font-medium">
        <Image24Regular className="text-cyan-300" style={{ fontSize: 16 }} />
        Screen captured
      </div>
    </div>
  );
};
