import { useDispatch, useSelector } from 'react-redux';
import { invoke } from '@tauri-apps/api/core';
import { 
  setCurrentModel, 
  setStatus, 
  setLoading, 
  setStreaming, 
  setLastResponse, 
  setModelsList, 
  setError, 
  setActiveRequestId,
  setConversationHistory,
  addActiveSource,
  clearActiveSources,
  setAutoCopyClipboard,
  addSessionMessage,
  clearSessionHistory
} from '../stores/ai.store';
import { aiService } from '../services/ai.service';
import { useStreaming } from './useStreaming';
import { buildPrompt } from '../utils/promptBuilder';
import { SYSTEM_PROMPT } from '../services/prompt.service';
import { conversationService } from '../conversation/conversation.service';
import { memoryManager } from '../conversation/memoryManager';
import type { PromptContext, AIError } from '../types/ai.types';

export const useAI = () => {
  const dispatch = useDispatch();
  const currentModel = useSelector((state: any) => state.ai.currentModel);
  const status = useSelector((state: any) => state.ai.status);
  const loading = useSelector((state: any) => state.ai.loading);
  const streaming = useSelector((state: any) => state.ai.streaming);
  const lastResponse = useSelector((state: any) => state.ai.lastResponse);
  const conversationHistory = useSelector((state: any) => state.ai.conversationHistory);
  const sessionHistory = useSelector((state: any) => state.ai.sessionHistory);
  const models = useSelector((state: any) => state.ai.modelsList);
  const error = useSelector((state: any) => state.ai.error);
  const activeRequestId = useSelector((state: any) => state.ai.activeRequestId);
  const autoCopyClipboard = useSelector((state: any) => state.ai.autoCopyClipboard);
  const activeSources = useSelector((state: any) => state.ai.activeSources);

  const { handleChunk } = useStreaming();

  const changeModel = (model: string) => {
    dispatch(setCurrentModel(model));
  };

  const getModels = async () => {
    try {
      const resp = await aiService.fetchModels();
      dispatch(setModelsList(resp.models));
      dispatch(setStatus('connected'));
      return resp.models;
    } catch (err: any) {
      dispatch(setStatus('error'));
      const aiErr = aiService.mapError(err);
      dispatch(setError(aiErr));
      return [];
    }
  };

  const healthCheck = async () => {
    try {
      const resp = await aiService.checkAvailability();
      dispatch(setStatus(resp.available ? 'connected' : 'disconnected'));
      return resp.available;
    } catch {
      dispatch(setStatus('disconnected'));
      return false;
    }
  };

  const loadHistory = async (limit = 20) => {
    const history = await conversationService.loadHistory(limit);
    dispatch(setConversationHistory(history));
  };

  const clearHistory = async () => {
    await conversationService.clearHistory();
    dispatch(setConversationHistory([]));
    dispatch(clearSessionHistory());
    dispatch(clearActiveSources());
  };

  const deleteMessage = async (id: number) => {
    try {
      await invoke('db_delete_message_cmd', { id });
      await loadHistory();
    } catch (err) {
      console.error('Failed to delete message:', err);
    }
  };

  const exportHistoryMarkdown = async (): Promise<string> => {
    try {
      return await invoke<string>('db_export_markdown_cmd');
    } catch (err) {
      console.error('Failed to export markdown:', err);
      return '';
    }
  };

  const exportHistoryJson = async (): Promise<string> => {
    try {
      return await invoke<string>('db_export_json_cmd');
    } catch (err) {
      console.error('Failed to export json:', err);
      return '';
    }
  };

  const setAutoCopy = (copy: boolean) => {
    dispatch(setAutoCopyClipboard(copy));
  };

  const verifyServerAndModel = async (): Promise<boolean> => {
    const isAvailable = await healthCheck();
    if (!isAvailable) {
      const err: AIError = {
        type: 'SERVER_UNAVAILABLE',
        message:
          'Gemini API is not configured. Add GEMINI_API_KEY to your .env file and restart the app.',
      };
      dispatch(setError(err));
      dispatch(setStatus('disconnected'));
      return false;
    }

    const availableModels = await getModels();
    if (availableModels.length === 0) {
      const err: AIError = {
        type: 'MODEL_NOT_FOUND',
        message: 'No Gemini models available. Check your API key and network connection.',
      };
      dispatch(setError(err));
      return false;
    }

    const targetModel = currentModel.toLowerCase();
    const modelExists = availableModels.some((m) => {
      const name = m.name.toLowerCase();
      const model = m.model.toLowerCase();
      return name === targetModel || model === targetModel;
    });

    if (!modelExists) {
      const err: AIError = {
        type: 'MODEL_NOT_FOUND',
        message: `Model '${currentModel}' is not available. Pick another Gemini model in Settings.`,
      };
      dispatch(setError(err));
      return false;
    }

    return true;
  };

  const ask = async (
    prompt: string,
    context: PromptContext = {},
    system?: string,
    options?: { temperature?: number; num_predict?: number }
  ): Promise<string> => {
    dispatch(setError(null));
    dispatch(setLastResponse(''));

    const isReady = await verifyServerAndModel();
    if (!isReady) {
      return '';
    }

    const requestId = crypto.randomUUID();
    dispatch(setLoading(true));
    dispatch(setActiveRequestId(requestId));

    // Update Context Badges based on active inputs
    if (context.ocrText && context.ocrText.trim().length > 0) {
      memoryManager.setOcrText(context.ocrText);
      dispatch(addActiveSource('OCR'));
    }
    if (context.voiceText && context.voiceText.trim().length > 0) {
      memoryManager.setVoiceText(context.voiceText);
      dispatch(addActiveSource('Voice'));
    }

    try {
      const history = await conversationService.loadHistory(5);
      const activeContext = memoryManager.getContext();
      const contextWithHistory = { ...activeContext, chatHistory: history };
      const builtPrompt = buildPrompt(prompt, contextWithHistory);
      
      const responseText = await aiService.ask(
        requestId,
        currentModel,
        builtPrompt,
        system || SYSTEM_PROMPT,
        options
      );

      dispatch(setLastResponse(responseText));
      
      // Save logs to SQLite
      await conversationService.saveMessage('user', prompt, 'chat');
      await conversationService.saveMessage('assistant', responseText, 'chat');
      
      dispatch(addSessionMessage({ role: 'user', content: prompt, id: Date.now(), timestamp: Date.now() }));
      dispatch(addSessionMessage({ role: 'assistant', content: responseText, id: Date.now() + 1, timestamp: Date.now() }));

      // Auto-copy response to clipboard if configured
      if (autoCopyClipboard) {
        navigator.clipboard.writeText(responseText).catch(e => {
          console.warn('Clipboard write failed:', e);
        });
      }

      // Reload database history
      await loadHistory();
      dispatch(setStatus('connected'));
      return responseText;
    } catch (err: any) {
      const aiErr = aiService.mapError(err);
      dispatch(setError(aiErr));
      if (aiErr.type === 'SERVER_UNAVAILABLE') {
        dispatch(setStatus('disconnected'));
      }
      throw aiErr;
    } finally {
      dispatch(setLoading(false));
      dispatch(setActiveRequestId(null));
    }
  };

  const stream = async (
    prompt: string,
    context: PromptContext = {},
    system?: string,
    options?: { temperature?: number; num_predict?: number }
  ): Promise<void> => {
    dispatch(setError(null));
    dispatch(setLastResponse(''));

    const isReady = await verifyServerAndModel();
    if (!isReady) {
      return;
    }

    const requestId = crypto.randomUUID();
    dispatch(setLoading(true));
    dispatch(setStreaming(true));
    dispatch(setActiveRequestId(requestId));

    if (context.ocrText && context.ocrText.trim().length > 0) {
      memoryManager.setOcrText(context.ocrText);
      dispatch(addActiveSource('OCR'));
    }
    if (context.voiceText && context.voiceText.trim().length > 0) {
      memoryManager.setVoiceText(context.voiceText);
      dispatch(addActiveSource('Voice'));
    }

    try {
      const history = await conversationService.loadHistory(5);
      const activeContext = memoryManager.getContext();
      const contextWithHistory = { ...activeContext, chatHistory: history };
      const builtPrompt = buildPrompt(prompt, contextWithHistory);

      let accumulated = '';
      await aiService.stream(
        requestId,
        currentModel,
        builtPrompt,
        system || SYSTEM_PROMPT,
        options,
        undefined,
        (chunk) => {
          accumulated += chunk;
          handleChunk(chunk);
        }
      );

      // Save user & assistant logs to database on stream complete
      await conversationService.saveMessage('user', prompt, 'chat');
      await conversationService.saveMessage('assistant', accumulated, 'chat');

      dispatch(addSessionMessage({ role: 'user', content: prompt, id: Date.now(), timestamp: Date.now() }));
      dispatch(addSessionMessage({ role: 'assistant', content: accumulated, id: Date.now() + 1, timestamp: Date.now() }));

      if (autoCopyClipboard) {
        navigator.clipboard.writeText(accumulated).catch(e => {
          console.warn('Clipboard write failed:', e);
        });
      }

      await loadHistory();
      dispatch(setStatus('connected'));
    } catch (err: any) {
      const aiErr = aiService.mapError(err);
      dispatch(setError(aiErr));
      if (aiErr.type === 'SERVER_UNAVAILABLE') {
        dispatch(setStatus('disconnected'));
      }
      throw aiErr;
    } finally {
      dispatch(setLoading(false));
      dispatch(setStreaming(false));
      dispatch(setActiveRequestId(null));
    }
  };

  const cancel = async () => {
    if (activeRequestId) {
      await aiService.cancel(activeRequestId);
      dispatch(setActiveRequestId(null));
      dispatch(setLoading(false));
      dispatch(setStreaming(false));
    }
  };

  /** Present a direct Gemini voice answer (no STT / no re-prompt). */
  const presentVoiceAnswer = async (answer: string, source?: string) => {
    const text = answer.trim();
    if (!text) return;

    dispatch(setError(null));
    dispatch(setLoading(false));
    dispatch(setStreaming(false));
    dispatch(setLastResponse(text));
    dispatch(
      addSessionMessage({
        role: 'assistant',
        content: text,
        id: Date.now(),
        source: source || 'voice',
        timestamp: Date.now(),
      })
    );

    try {
      await conversationService.saveMessage('assistant', text, source || 'voice');
    } catch {
      // non-fatal
    }

    if (autoCopyClipboard) {
      navigator.clipboard.writeText(text).catch(() => {});
    }

    dispatch(setStatus('connected'));
  };

  return {
    ask,
    stream,
    cancel,
    presentVoiceAnswer,
    healthCheck,
    getModels,
    loadHistory,
    clearHistory,
    deleteMessage,
    exportHistoryMarkdown,
    exportHistoryJson,
    setAutoCopy,
    autoCopyClipboard,
    activeSources,
    loading,
    streaming,
    response: lastResponse,
    error,
    status,
    models,
    currentModel,
    changeModel,
    conversationHistory,
    sessionHistory,
    setError: (err: AIError | null) => dispatch(setError(err)),
  };
};
