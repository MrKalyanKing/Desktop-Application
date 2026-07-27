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
  setAutoCopyClipboard
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
        message: 'Ollama is not running. Please make sure the Ollama desktop app is active.'
      };
      dispatch(setError(err));
      dispatch(setStatus('disconnected'));
      return false;
    }

    const installedModels = await getModels();
    if (installedModels.length === 0) {
      const err: AIError = {
        type: 'MODEL_NOT_FOUND',
        message: 'No models installed in Ollama. Pull a model (e.g. "ollama pull llama3") first.'
      };
      dispatch(setError(err));
      return false;
    }

    const targetModel = currentModel.toLowerCase();
    const modelExists = installedModels.some(m => {
      const name = m.name.toLowerCase();
      const model = m.model.toLowerCase();
      return name === targetModel || 
             model === targetModel || 
             name.startsWith(targetModel + ':') || 
             targetModel.startsWith(name + ':') ||
             name.split(':')[0] === targetModel.split(':')[0];
    });

    if (!modelExists) {
      const err: AIError = {
        type: 'MODEL_NOT_FOUND',
        message: `Model '${currentModel}' is not pulled. Run 'ollama pull ${currentModel}' in your terminal.`
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

  return {
    ask,
    stream,
    cancel,
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
    setError: (err: AIError | null) => dispatch(setError(err)),
  };
};
