import React, { useEffect } from 'react';
import { StoreProvider } from './providers/StoreProvider';
import { FloatingWidget } from '../features/widget';
import { opsService } from '../features/hotkeys';
import { RegionSelector } from '../features/screenshot';
import { useAutoResize } from '../features/widget';
import { LocalOnlyCursor } from '../shared/components/LocalOnlyCursor';

const AppContent: React.FC = () => {
  const isRegionSelector = window.location.pathname.includes('region-selector') || 
                           window.location.hash.includes('region-selector');

  useEffect(() => {
    document.documentElement.classList.toggle('region-selector-mode', isRegionSelector);
    return () => document.documentElement.classList.remove('region-selector-mode');
  }, [isRegionSelector]);

  // Global shortcuts are registered in Rust; this window only tracks F3 cancellations.
  useEffect(() => {
    if (!isRegionSelector) opsService.init();
  }, [isRegionSelector]);

  if (isRegionSelector) {
    return <RegionSelector />;
  }

  // Resize window dynamically based on actual element dimensions
  useAutoResize();

  return (
    <div className="flex h-screen w-screen items-center justify-center overflow-hidden bg-transparent p-2">
      <FloatingWidget />
      <LocalOnlyCursor />
    </div>
  );
};

export const App: React.FC = () => {
  return (
    <StoreProvider>
      <AppContent />
    </StoreProvider>
  );
};

export default App;
