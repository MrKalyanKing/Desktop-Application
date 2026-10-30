import React, { useEffect, useMemo, useState, useRef } from 'react';
import { useDispatch, useStore } from 'react-redux';
import { invoke } from '@tauri-apps/api/core';
import {
  Camera24Regular,
  Search24Regular,
  Sparkle24Filled,
  Bot24Regular,
  Person24Regular,
  Mic24Filled,
  MicOff24Regular,
  Settings24Regular,
  Clipboard24Regular,
  CheckmarkCircle24Filled,
  History24Regular,
  ArrowUp24Filled,
  Dismiss24Regular,
  ArrowLeft24Regular,
  Delete24Regular,
  DocumentText24Regular,
  Code24Regular,
  Warning24Filled,
  Image24Regular,
} from '@fluentui/react-icons';
import { AIStatus, useAI } from '../../ai';
import { CapturePreview, useScreenshot } from '../../screenshot';
import { useSystemTray } from '../../tray';
import { SettingsPanel } from '../../settings';
import { useVoiceAgent } from '../hooks/useVoiceAgent';
import { ModeToggle } from './ModeToggle';
import { ListeningPanel } from './ListeningPanel';
import { AnswerView, toExchanges, type Exchange } from './AnswerView';
import { MarkdownRenderer } from '../../../shared/components/MarkdownRenderer';
import { GlassKeyCombo } from '../../../shared/components/glass';
import { useShortcutActions, useShortcutFeedback, showNotice, DOUBLE_PRESS_DELAY } from '../../hotkeys';
import { setLoading, setStreaming, setLastResponse, setActiveRequestId } from '../../ai/stores/ai.store';
import { widgetService } from '../services/widget.service';
import { useLocalStorage } from '../../../shared/hooks/useLocalStorage';

const SCROLL_STEP = 60;
const ANSWER_SIZES = [12, 13, 14, 15, 16, 18, 20, 22];


const isTextField = (el: Element | null) =>
  !!el && (el.tagName === 'TEXTAREA' || el.tagName === 'INPUT' || (el as HTMLElement).isContentEditable);

export const WidgetContent: React.FC = () => {
  const dispatch = useDispatch();
  const store = useStore();
  const [inputVal, setInputVal] = useState('');
  const [showSettings, setShowSettings] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [viewMode, setViewMode] = useState<'chat' | 'history'>('chat');
  const [answerSize, setAnswerSize] = useLocalStorage<number>('ks.answerSize', 15);
  const stepAnswerSize = (dir: 1 | -1) => {
    const i = ANSWER_SIZES.findIndex((v) => v >= answerSize);
    const next = ANSWER_SIZES[Math.min(ANSWER_SIZES.length - 1, Math.max(0, (i < 0 ? 2 : i) + dir))];
    setAnswerSize(next);
  };

  const {
    stream,
    cancel,
    loading,
    streaming,
    response,
    error,
    currentModel,
    loadHistory,
    clearHistory,
    deleteMessage,
    exportHistoryMarkdown,
    exportHistoryJson,
    setAutoCopy,
    autoCopyClipboard,
    activeSources,
    healthCheck,
    getModels,
    conversationHistory,
    sessionHistory
  } = useAI();

  const {
    isRecording,
    systemQuestion,
    voicePhase,
    health,
    speakingSource,
    heard,
    latency,
    note,
    voiceError,
    captureMode,
    actualSources,
    setCaptureMode,
    cycleCaptureMode,
    toggleVoice,
  } = useVoiceAgent();

  useEffect(() => {
    if (systemQuestion) setInputVal(systemQuestion);
  }, [systemQuestion]);

  const {
    step: screenshotStep,
    error: screenshotError,
    captureFullscreen,
    captureActiveWindow,
    captureRegion,
    startScrollCapture,
    openRegionSelector,
  } = useScreenshot();

  const { notice, failedKeys } = useShortcutFeedback();

  const scrollRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const lastQuestionRef = useRef<string>('');
  const lastEscapeRef = useRef(0);
  const [typedQuestion, setTypedQuestion] = useState<string | null>(null);

  useSystemTray({
    onOpenSettings: () => setShowSettings(true),
    onCheckStatus: () => { healthCheck(); },
    onRestartConnection: () => { getModels(); }
  });

  /** Ask a typed question; the live card shows it while the answer streams. */
  const askTyped = async (query: string) => {
    lastQuestionRef.current = query;
    setTypedQuestion(query);
    try {
      await stream(query);
    } finally {
      setTypedQuestion(null);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputVal.trim() || loading) return;

    const query = inputVal.trim();
    setInputVal('');
    try {
      await askTyped(query);
    } catch (err) {
      console.error('Streaming error:', err);
    }
  };

  const scrollAnswer = (key: string) => {
    const el = scrollRef.current;
    if (!el) return false;
    const page = Math.max(SCROLL_STEP, el.clientHeight * 0.9);
    const delta: Record<string, number> = {
      ArrowUp: -SCROLL_STEP,
      ArrowDown: SCROLL_STEP,
      PageUp: -page,
      PageDown: page,
    };
    if (key === 'Home') el.scrollTop = 0;
    else if (key === 'End') el.scrollTop = el.scrollHeight;
    else if (key in delta) el.scrollTop += delta[key];
    else return false;
    return true;
  };

  /** Esc: hand focus back to the app that was in front before F2 (best-effort). */
  const releaseFocus = async () => {
    textareaRef.current?.blur();
    const now = Date.now();
    const isDouble = now - lastEscapeRef.current < DOUBLE_PRESS_DELAY;
    lastEscapeRef.current = now;
    try {
      const result = await invoke<string>('release_focus_cmd');
      if (result === 'failed') showNotice('error', "Couldn't return focus to the previous app");
      if (result !== 'restored' && isDouble) await widgetService.hide();
    } catch {
      if (isDouble) await widgetService.hide();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
      return;
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      releaseFocus();
      return;
    }
    // Page keys always scroll the answer; arrows/Home/End only when there is no text
    // to edit, so cursor movement and selection keep working normally.
    const pageKey = e.key === 'PageUp' || e.key === 'PageDown';
    const navKey = ['ArrowUp', 'ArrowDown', 'Home', 'End'].includes(e.key);
    if ((pageKey || (navKey && inputVal.length === 0)) && !e.altKey && !e.ctrlKey && !e.metaKey) {
      if (scrollAnswer(e.key)) e.preventDefault();
    }
  };

  // Overlay focused but not typing (e.g. answer pane): scroll keys and Esc.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (isTextField(document.activeElement)) return;
      if (e.altKey || e.ctrlKey || e.metaKey) return;
      if (e.key === 'Escape') {
        releaseFocus();
        return;
      }
      if (showSettings || viewMode !== 'chat') return;
      if (scrollAnswer(e.key)) e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  useShortcutActions({
    onAction: ({ action, targetHwnd }) => {
      switch (action) {
        case 'focus-input':
          setShowSettings(false);
          setViewMode('chat');
          requestAnimationFrame(() => textareaRef.current?.focus());
          break;
        case 'regenerate': {
          const s = (store.getState() as any).ai;
          if (s.loading || s.streaming) return;
          const q = lastQuestionRef.current;
          if (!q) {
            showNotice('info', 'No previous question');
            return;
          }
          askTyped(q).catch((err) => console.error('Regenerate failed:', err));
          break;
        }
        case 'capture-full':
          captureFullscreen();
          break;
        case 'capture-window':
          captureActiveWindow(targetHwnd ?? null);
          break;
        case 'capture-region':
          openRegionSelector();
          break;
        case 'capture-scroll':
          if (targetHwnd != null) startScrollCapture(targetHwnd);
          break;
        case 'toggle-voice':
          toggleVoice();
          break;
        case 'cycle-audio':
          cycleCaptureMode();
          break;
      }
    },
    onRegionSelected: ({ x, y, width, height }) => captureRegion(x, y, width, height),
    onCancelled: (ids) => {
      if (ids.length === 0) return;
      const s = (store.getState() as any).ai;
      const typedCancelled = s.activeRequestId != null && ids.includes(Number(s.activeRequestId));
      const voicePartial = s.activeRequestId == null && s.streaming;
      if (typedCancelled || voicePartial) {
        dispatch(setLoading(false));
        dispatch(setStreaming(false));
        dispatch(setLastResponse(''));
        if (typedCancelled) dispatch(setActiveRequestId(null));
      }
    },
  });

  const handleExportMd = async () => {
    const data = await exportHistoryMarkdown();
    if (data) navigator.clipboard.writeText(data).catch(() => { });
  };

  const handleExportJson = async () => {
    const data = await exportHistoryJson();
    if (data) navigator.clipboard.writeText(data).catch(() => { });
  };

  // Voice state lives in the listening panel; this badge covers screenshots only.
  const getStatusBadge = () => {
    if (screenshotStep === 'capturing' || screenshotStep === 'ai') {
      return (
        <span className="inline-flex items-center gap-0.5 text-[8px] font-semibold text-amber-300 bg-amber-500/10 px-1.5 py-0 rounded-full border border-amber-400/20 leading-tight">
          <Camera24Regular style={{ fontSize: 10 }} />
          {screenshotStep === 'capturing' ? 'Capture' : 'Reading'}
        </span>
      );
    }
    if (screenshotStep === 'error') {
      return (
        <span className="inline-flex items-center gap-0.5 text-[8px] font-semibold text-rose-300 bg-rose-500/10 px-1.5 py-0 rounded-full border border-rose-400/20 leading-tight">
          <Warning24Filled style={{ fontSize: 10 }} /> Error
        </span>
      );
    }
    return null;
  };

  const exchanges = useMemo(() => toExchanges(sessionHistory), [sessionHistory]);
  const voiceLive = !typedQuestion && (streaming || voicePhase === 'thinking');
  const liveKeyRef = useRef({ active: false, key: '' });
  const isLive = !!typedQuestion || voiceLive;
  if (isLive && !liveKeyRef.current.active) {
    liveKeyRef.current = { active: true, key: `live-${Date.now()}` };
  } else if (!isLive && liveKeyRef.current.active) {
    liveKeyRef.current = { active: false, key: '' };
  }
  const liveExchange: Exchange | null = isLive
    ? {
        key: liveKeyRef.current.key,
        question: typedQuestion ?? heard?.text,
        answer: response || undefined,
        source: typedQuestion ? 'chat' : heard?.source || 'voice',
      }
    : null;

  const filteredHistory = conversationHistory.filter((msg: any) =>
    msg.content.toLowerCase().includes(searchQuery.toLowerCase())
  );

  if (showSettings) {
    return (
      <div className="flex-1 min-h-0 overflow-hidden flex flex-col w-full">
        <SettingsPanel onClose={() => setShowSettings(false)} />
      </div>
    );
  }

  return (
    <div className="flex-1 min-h-0 px-2.5 pb-2 pt-1.5 flex flex-col text-[12px] overflow-hidden w-full">
      <div className="flex items-center justify-between mb-1.5 select-none gap-1.5 shrink-0">
        <div className="flex items-center gap-1 min-w-0 flex-wrap">
          <AIStatus />
          {getStatusBadge()}
          {activeSources.map((src: string) => (
            <span key={src} className="lg-chip">
              {src === 'Voice' ? <Mic24Filled style={{ fontSize: 9 }} /> : <Image24Regular style={{ fontSize: 9 }} />}
              {src}
            </span>
          ))}
          {failedKeys.map((f) => (
            <span
              key={f.key}
              className="inline-flex items-center gap-0.5 text-[8px] font-semibold text-amber-300 bg-amber-500/10 px-1.5 py-0 rounded-full border border-amber-400/20 leading-tight"
              title={f.error}
            >
              <Warning24Filled style={{ fontSize: 10 }} /> {f.key} unavailable
            </span>
          ))}
        </div>
        <div className="flex items-center gap-0.5 shrink-0">
          <button
            type="button"
            onClick={() => stepAnswerSize(-1)}
            disabled={answerSize <= ANSWER_SIZES[0]}
            className="lg-icon-btn ks-size-btn"
            title="Smaller answer text"
          >
            A−
          </button>
          <button
            type="button"
            onClick={() => stepAnswerSize(1)}
            disabled={answerSize >= ANSWER_SIZES[ANSWER_SIZES.length - 1]}
            className="lg-icon-btn ks-size-btn"
            title="Larger answer text"
          >
            A+
          </button>
          <button
            type="button"
            onClick={toggleVoice}
            className={`lg-icon-btn ${isRecording ? 'is-live' : ''}`}
            title={isRecording ? 'Stop listening' : 'Start voice'}
          >
            {isRecording ? <MicOff24Regular style={{ fontSize: 12 }} /> : <Mic24Filled style={{ fontSize: 12 }} />}
          </button>
          <button
            type="button"
            onClick={() => setShowSettings(!showSettings)}
            className={`lg-icon-btn ${showSettings ? 'is-on' : ''}`}
            title="Settings"
          >
            <Settings24Regular style={{ fontSize: 12 }} />
          </button>
          <button
            type="button"
            onClick={() => setAutoCopy(!autoCopyClipboard)}
            className={`lg-icon-btn ${autoCopyClipboard ? 'is-on' : ''}`}
            title={autoCopyClipboard ? 'Auto-copy on' : 'Auto-copy off'}
          >
            {autoCopyClipboard ? (
              <CheckmarkCircle24Filled style={{ fontSize: 12 }} />
            ) : (
              <Clipboard24Regular style={{ fontSize: 12 }} />
            )}
          </button>
          <button
            type="button"
            onClick={() => {
              setViewMode(viewMode === 'history' ? 'chat' : 'history');
              if (viewMode !== 'history') loadHistory();
            }}
            className={`lg-icon-btn ${viewMode === 'history' ? 'is-on' : ''}`}
            title="History"
          >
            <History24Regular style={{ fontSize: 12 }} />
          </button>
        </div>
      </div>

      <div className="flex-1 min-h-0 flex flex-col">
        {viewMode === 'history' ? (
          <div className="flex flex-col flex-1 min-h-0">
            <div className="flex justify-between items-center mb-2 select-none">
              <span className="inline-flex items-center gap-1.5 text-[12px] font-semibold text-slate-100">
                <History24Regular style={{ fontSize: 16 }} className="text-cyan-300" />
                History
              </span>
              <button
                type="button"
                onClick={() => setViewMode('chat')}
                className="lg-btn h-7 px-2.5 text-[11px]"
              >
                <ArrowLeft24Regular style={{ fontSize: 14 }} />
                Chat
              </button>
            </div>

            <div className="flex gap-1 mb-2 items-center select-none">
              <div className="lg-input-bar h-9 min-h-9 flex-1 px-3">
                <Search24Regular className="mr-1.5 text-[var(--lg-text-muted)]" style={{ fontSize: 14 }} />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search…"
                  className="flex-1 bg-transparent text-[11px] text-slate-200 placeholder-slate-500 outline-none"
                />
              </div>
              <button type="button" onClick={handleExportMd} className="lg-icon-btn" title="Copy Markdown">
                <DocumentText24Regular style={{ fontSize: 15 }} />
              </button>
              <button type="button" onClick={handleExportJson} className="lg-icon-btn" title="Copy JSON">
                <Code24Regular style={{ fontSize: 15 }} />
              </button>
              <button type="button" onClick={clearHistory} className="lg-icon-btn" title="Clear history">
                <Delete24Regular style={{ fontSize: 15 }} />
              </button>
            </div>

            <div
              ref={scrollRef}
              className="lg-card min-h-0 flex-1 overflow-y-auto px-2 py-2 text-[var(--lg-text-secondary)]"
            >
              {filteredHistory.length === 0 && (
                <div className="text-center py-8 text-slate-500 text-[11px]">No saved messages yet.</div>
              )}
              {filteredHistory.map((msg: any) => (
                <div key={msg.id} className="mb-2.5 relative group">
                  <div className="flex justify-between items-center text-[10px] font-semibold text-slate-500 mb-1">
                    <span className="inline-flex items-center gap-1">
                      {msg.role === 'user' ? <Person24Regular style={{ fontSize: 12 }} /> : <Bot24Regular style={{ fontSize: 12 }} />}
                      {msg.role === 'user' ? 'You' : 'KS Vision'}
                    </span>
                    <button
                      type="button"
                      onClick={() => msg.id && deleteMessage(msg.id)}
                      className="opacity-0 group-hover:opacity-100 text-slate-500 hover:text-rose-400"
                      title="Delete"
                    >
                      <Dismiss24Regular style={{ fontSize: 12 }} />
                    </button>
                  </div>
                  <div className="lg-card lg-chat p-3">
                    <MarkdownRenderer content={msg.content} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="flex flex-col flex-1 min-h-0 gap-2">
            {isRecording && (
              <ListeningPanel
                actualSources={actualSources}
                phase={voicePhase}
                speakingSource={speakingSource}
                health={health}
                latency={latency}
                note={note}
                voiceError={voiceError}
              />
            )}

            <div
              ref={scrollRef}
              tabIndex={0}
              role="region"
              aria-label="Answer"
              className="ks-answer-pane min-h-0 flex-1 overflow-y-auto px-0.5 py-1 text-[var(--lg-text-secondary)]"
              style={{ ['--ks-answer-size' as string]: `${answerSize}px` }}
            >
              <CapturePreview />

              {exchanges.length === 0 && !isLive && !error && !screenshotError && (
                <div className="space-y-3 select-none py-1">
                  <div className="lg-card p-3.5 lg-chat">
                    <div className="mb-1 flex items-center gap-2 text-[14px] font-semibold text-[var(--lg-text)]">
                      <Sparkle24Filled className="text-[var(--lg-accent)]" style={{ fontSize: 18 }} />
                      Ready when you are
                    </div>
                    <p className="text-[12px] leading-relaxed text-[var(--lg-text-secondary)]">
                      Ask in the box, tap the mic, or capture your screen. Answers stay glanceable.
                    </p>
                    {currentModel && (
                      <p className="mt-2 truncate font-mono text-[10px] text-[var(--lg-text-muted)]">{currentModel}</p>
                    )}
                  </div>

                  <div className="lg-stack">
                    {[
                      ['Ask (focus input) · also Ctrl+Shift+S', 'F2'],
                      ['Screen / window / region / scroll', 'F6 F7 F8 F9'],
                      ['Voice on / off · source', 'F10 F11'],
                      ['Stop · copy · regenerate', 'F3 F4 F5'],
                      ['Show / hide overlay (ghost mode)', 'Ctrl+Shift+E'],
                      ['Quit completely', 'Ctrl+Shift+P'],
                    ].map(([label, keys]) => (
                      <div key={keys} className="lg-row">
                        <span>{label}</span>
                        <GlassKeyCombo value={keys} />
                      </div>
                    ))}
                  </div>
                  <p className="pt-1 text-center text-[10px] text-[var(--lg-text-muted)]">Esc returns to your app · Alt+Arrows move the overlay.</p>
                </div>
              )}

              {(error || screenshotError) && (
                <div className="flex gap-2 p-2.5 rounded-xl bg-rose-500/10 border border-rose-400/20 text-rose-200 text-[11px] mt-1">
                  <Warning24Filled style={{ fontSize: 16 }} className="shrink-0 mt-0.5" />
                  <div>
                    <div className="font-semibold mb-0.5">Something went wrong</div>
                    {error?.message || screenshotError}
                  </div>
                </div>
              )}

              <AnswerView exchanges={exchanges} live={liveExchange} thinking={isLive && !response} />
            </div>
          </div>
        )}
      </div>

      {viewMode === 'chat' && (
        <form onSubmit={handleSubmit} className="mt-1 flex w-full min-w-0 shrink-0 flex-col gap-1.5 select-none">
          {notice && (
            <div
              role="status"
              aria-live="polite"
              className={`truncate px-1 text-[10px] ${
                notice.level === 'error'
                  ? 'text-rose-300'
                  : notice.level === 'success'
                    ? 'text-emerald-300'
                    : 'text-[var(--lg-text-muted)]'
              }`}
              title={notice.message}
            >
              {notice.message}
            </div>
          )}
          <div className="lg-input-bar">
            <textarea
              ref={textareaRef}
              value={inputVal}
              onChange={(e) => setInputVal(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={loading ? 'Thinking…' : 'Ask anything…'}
              disabled={loading && !streaming}
              rows={2}
              className="h-[36px] flex-1 py-2 leading-snug disabled:opacity-50"
            />
            {loading ? (
              <button type="button" onClick={cancel} className="lg-send is-stop" title="Stop">
                <Dismiss24Regular style={{ fontSize: 14 }} />
              </button>
            ) : (
              <button type="submit" disabled={!inputVal.trim()} className="lg-send" title="Send">
                <ArrowUp24Filled style={{ fontSize: 15 }} />
              </button>
            )}
          </div>

          <ModeToggle mode={captureMode} onChange={setCaptureMode} />
        </form>
      )}
    </div>
  );
};
