import React from 'react';
import { StoreProvider } from './providers/StoreProvider';
import { FloatingWidget } from '../features/widget';
import { useGlobalHotkeys } from '../features/hotkeys';
import { RegionSelector } from '../features/screenshot';
import { useAutoResize } from '../features/widget';

const AppContent: React.FC = () => {
  const isRegionSelector = window.location.pathname.includes('region-selector') || 
                           window.location.hash.includes('region-selector');

  if (isRegionSelector) {
    return <RegionSelector />;
  }

  // Initialize global shortcut (Ctrl+Shift+H toggle) and local Escape hooks
  useGlobalHotkeys();

  // Resize window dynamically based on actual element dimensions
  useAutoResize();

  return (
    <div className="flex h-screen w-screen items-center justify-center bg-transparent overflow-hidden">
      <FloatingWidget />
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
