import { useEffect } from 'react';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

export const useAutoResize = () => {
  useEffect(() => {
    const resizeWindow = async () => {
      try {
        const win = getCurrentWindow();
        
        // Measure exact height of the body content
        const bodyHeight = document.body.scrollHeight;
        
        // Set limits: min 150px (idle), max 650px (expanded)
        const targetHeight = Math.min(650, Math.max(150, bodyHeight));
        
        const currentSize = await win.outerSize();
        const factor = await win.scaleFactor();
        const currentLogicalHeight = currentSize.height / factor;

        // Apply size changes only if there's a meaningful difference
        if (Math.abs(currentLogicalHeight - targetHeight) > 2) {
          await win.setSize(new LogicalSize(400, targetHeight));
        }
      } catch (err) {
        console.warn('Failed to resize window dynamically:', err);
      }
    };

    const observer = new ResizeObserver(() => {
      resizeWindow();
    });
    
    observer.observe(document.body);
    resizeWindow();

    return () => {
      observer.disconnect();
    };
  }, []);
};
