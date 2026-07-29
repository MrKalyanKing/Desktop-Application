import { createSlice } from '@reduxjs/toolkit';
import type { PayloadAction } from '@reduxjs/toolkit';
import type { AIStatusType, ChatMessage, AIError, AiModel } from '../types/ai.types';
import { DEFAULT_MODEL, STORAGE_KEYS } from '../constants/ai.constants';
import { storage } from '../../../shared/utils/storage';

interface AIState {
  currentModel: string;
  status: AIStatusType;
  loading: boolean;
  streaming: boolean;
  lastResponse: string;
  conversationHistory: ChatMessage[];
  sessionHistory: ChatMessage[];
  modelsList: AiModel[];
  error: AIError | null;
  activeRequestId: string | null;
  autoCopyClipboard: boolean;
  activeSources: string[];
}

const getInitialModel = (): string => {
  const model = storage.get(STORAGE_KEYS.ACTIVE_MODEL, DEFAULT_MODEL) as string;
  return model || DEFAULT_MODEL;
};

const initialState: AIState = {
  currentModel: getInitialModel(),
  status: 'disconnected',
  loading: false,
  streaming: false,
  lastResponse: '',
  conversationHistory: [],
  sessionHistory: [],
  modelsList: [],
  error: null,
  activeRequestId: null,
  autoCopyClipboard: true,
  activeSources: [],
};

export const aiSlice = createSlice({
  name: 'ai',
  initialState,
  reducers: {
    setCurrentModel: (state, action: PayloadAction<string>) => {
      state.currentModel = action.payload;
      storage.set(STORAGE_KEYS.ACTIVE_MODEL, action.payload);
    },
    setStatus: (state, action: PayloadAction<AIStatusType>) => {
      state.status = action.payload;
    },
    setLoading: (state, action: PayloadAction<boolean>) => {
      state.loading = action.payload;
    },
    setStreaming: (state, action: PayloadAction<boolean>) => {
      state.streaming = action.payload;
    },
    setLastResponse: (state, action: PayloadAction<string>) => {
      state.lastResponse = action.payload;
    },
    appendLastResponse: (state, action: PayloadAction<string>) => {
      state.lastResponse += action.payload;
    },
    setModelsList: (state, action: PayloadAction<AiModel[]>) => {
      state.modelsList = action.payload;
    },
    setError: (state, action: PayloadAction<AIError | null>) => {
      state.error = action.payload;
    },
    setActiveRequestId: (state, action: PayloadAction<string | null>) => {
      state.activeRequestId = action.payload;
    },
    addChatMessage: (state, action: PayloadAction<ChatMessage>) => {
      state.conversationHistory.push(action.payload);
    },
    clearChatHistory: (state) => {
      state.conversationHistory = [];
    },
    setConversationHistory: (state, action: PayloadAction<ChatMessage[]>) => {
      state.conversationHistory = action.payload;
    },
    setAutoCopyClipboard: (state, action: PayloadAction<boolean>) => {
      state.autoCopyClipboard = action.payload;
    },
    setActiveSources: (state, action: PayloadAction<string[]>) => {
      state.activeSources = action.payload;
    },
    addActiveSource: (state, action: PayloadAction<string>) => {
      if (!state.activeSources.includes(action.payload)) {
        state.activeSources.push(action.payload);
      }
    },
    clearActiveSources: (state) => {
      state.activeSources = [];
    },
    addSessionMessage: (state, action: PayloadAction<ChatMessage>) => {
      state.sessionHistory.push(action.payload);
    },
    clearSessionHistory: (state) => {
      state.sessionHistory = [];
    },
  },
});

export const {
  setCurrentModel,
  setStatus,
  setLoading,
  setStreaming,
  setLastResponse,
  appendLastResponse,
  setModelsList,
  setError,
  setActiveRequestId,
  addChatMessage,
  clearChatHistory,
  setConversationHistory,
  setAutoCopyClipboard,
  setActiveSources,
  addActiveSource,
  clearActiveSources,
  addSessionMessage,
  clearSessionHistory,
} = aiSlice.actions;

export default aiSlice.reducer;
