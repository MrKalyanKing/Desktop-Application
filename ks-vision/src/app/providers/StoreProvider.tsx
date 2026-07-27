import React from 'react';
import { Provider } from 'react-redux';
import { configureStore } from '@reduxjs/toolkit';
import { widgetReducer } from '../../features/widget';
import { aiReducer } from '../../features/ai';
import { screenshotReducer } from '../../features/screenshot';
import { settingsReducer } from '../../features/settings';

export const store = configureStore({
  reducer: {
    widget: widgetReducer,
    ai: aiReducer,
    screenshot: screenshotReducer,
    settings: settingsReducer,
  },
});

export type RootState = ReturnType<typeof store.getState>;
export type AppDispatch = typeof store.dispatch;

interface StoreProviderProps {
  children: React.ReactNode;
}

export const StoreProvider: React.FC<StoreProviderProps> = ({ children }) => {
  return <Provider store={store}>{children}</Provider>;
};
