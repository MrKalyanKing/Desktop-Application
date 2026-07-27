import React from 'react';
import { useSelector } from 'react-redux';

export const CapturePreview: React.FC = () => {
  const preview = useSelector((state: any) => state.screenshot.currentPreview);
  const ocrResult = useSelector((state: any) => state.screenshot.lastOcrResult);

  if (!preview) return null;

  return (
    <div className="flex gap-2 bg-slate-950/20 p-1.5 rounded-lg border border-slate-850/40 select-none my-0.5">
      <div className="relative w-12 h-12 bg-slate-950 border border-slate-800/40 rounded overflow-hidden flex-shrink-0 flex items-center justify-center">
        <img 
          src={preview} 
          alt="Capture preview"
          className="max-w-full max-h-full object-contain"
        />
      </div>
      
      {ocrResult && (
        <div className="flex-1 flex flex-col justify-center text-[9px] text-slate-400">
          <div className="flex items-center justify-between">
            <span className="font-bold text-slate-300 tracking-wider text-[8px] uppercase">
              {ocrResult.contentType === 'Unknown' ? 'Screen Image' : ocrResult.contentType}
            </span>
            <span className="font-mono text-cyan-400 font-bold">{ocrResult.confidence}% confidence</span>
          </div>
          <div className="flex items-center justify-between mt-1 text-[8px]">
            <span>Lang: <strong className="text-slate-300">{ocrResult.language}</strong></span>
            <span className="text-[8px] font-mono text-slate-500">{ocrResult.width}x{ocrResult.height} px</span>
          </div>
          <div className="text-[8px] font-mono text-slate-500 mt-0.5 text-right">
            Processed in {ocrResult.captureTimeMs}ms
          </div>
        </div>
      )}
    </div>
  );
};
