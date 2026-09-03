import React, { useEffect, useState, useRef } from 'react';
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
  Keyboard24Regular,
} from '@fluentui/react-icons';
import { AIStatus, AIThinking, useAI } from '../../ai';
import { CapturePreview, useScreenshot } from '../../screenshot';
import { useSystemTray } from '../../tray';
import { SettingsPanel } from '../../settings';
import { useVoiceAgent } from '../hooks/useVoiceAgent';
import { ModeToggle } from './ModeToggle';
import { AudioIntelligenceUI } from './AudioIntelligenceUI';
import { MarkdownRenderer } from '../../../shared/components/MarkdownRenderer';

const Kbd: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <kbd className="ks-kbd">{children}</kbd>
);

export const WidgetContent: React.FC = () => {
  const [inputVal, setInputVal] = useState('');
  const [showSettings, setShowSettings] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [viewMode, setViewMode] = useState<'chat' | 'history'>('chat');

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

  const { isRecording, isTranscribing, systemQuestion, usingScreen, captureMode, setCaptureMode, toggleVoice } = useVoiceAgent();

  useEffect(() => {
    if (systemQuestion) setInputVal(systemQuestion);
  }, [systemQuestion]);

  const {
    step: screenshotStep,
    error: screenshotError
  } = useScreenshot();

  const scrollRef = useRef<HTMLDivElement>(null);
  const answerStartRef = useRef<HTMLDivElement>(null);
  const userScrolledAwayRef = useRef(false);
  const prevLoadingRef = useRef(false);

  useSystemTray({
    onOpenSettings: () => setShowSettings(true),
    onCheckStatus: () => { healthCheck(); },
    onRestartConnection: () => { getModels(); }
  });

  const handleChatScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const distFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    userScrolledAwayRef.current = distFromBottom > 64;
  };

  useEffect(() => {
    const justStarted = loading && !prevLoadingRef.current;
    prevLoadingRef.current = loading;

    if (justStarted) {
      userScrolledAwayRef.current = false;
      requestAnimationFrame(() => {
        answerStartRef.current?.scrollIntoView({ block: 'start', behavior: 'smooth' });
      });
      return;
    }

    if (streaming || loading) return;
    if (userScrolledAwayRef.current) return;

    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [conversationHistory, sessionHistory, loading, streaming, error, screenshotStep, screenshotError, viewMode]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!inputVal.trim() || loading) return;

    const query = inputVal.trim();
    setInputVal('');
    try {
      await stream(query);
    } catch (err) {
      console.error('Streaming error:', err);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e);
    }
  };

  const handleExportMd = async () => {
    const data = await exportHistoryMarkdown();
    if (data) navigator.clipboard.writeText(data).catch(() => { });
  };

  const handleExportJson = async () => {
    const data = await exportHistoryJson();
    if (data) navigator.clipboard.writeText(data).catch(() => { });
  };

  const getStatusBadge = () => {
    if (isRecording) {
      return (
        <span className="inline-flex items-center gap-0.5 text-[8px] font-semibold text-rose-300 bg-rose-500/10 px-1.5 py-0 rounded-full border border-rose-400/20 leading-tight">
          <Mic24Filled style={{ fontSize: 10 }} /> Live
        </span>
      );
    }
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
            <span key={src} className="inline-flex items-center gap-0.5 text-[8px] font-semibold text-sky-300 bg-sky-500/10 px-1.5 py-0 rounded-full border border-sky-400/20 leading-tight">
              {src === 'Voice' ? <Mic24Filled style={{ fontSize: 9 }} /> : <Image24Regular style={{ fontSize: 9 }} />}
              {src}
            </span>
          ))}
        </div>
        <div className="flex items-center gap-0.5 shrink-0">
          <button
            type="button"
            onClick={toggleVoice}
            className={`ks-icon-btn ${isRecording ? 'is-live' : ''}`}
            title={isRecording ? 'Stop listening' : 'Start voice'}
          >
            {isRecording ? <MicOff24Regular style={{ fontSize: 12 }} /> : <Mic24Filled style={{ fontSize: 12 }} />}
          </button>
          <button
            type="button"
            onClick={() => setShowSettings(!showSettings)}
            className={`ks-icon-btn ${showSettings ? 'is-on' : ''}`}
            title="Settings"
          >
            <Settings24Regular style={{ fontSize: 12 }} />
          </button>
          <button
            type="button"
            onClick={() => setAutoCopy(!autoCopyClipboard)}
            className={`ks-icon-btn ${autoCopyClipboard ? 'is-on' : ''}`}
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
            className={`ks-icon-btn ${viewMode === 'history' ? 'is-on' : ''}`}
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
                className="inline-flex items-center gap-1 h-7 px-2.5 rounded-lg bg-white/5 border border-white/10 text-slate-300 hover:text-white text-[11px] font-semibold"
              >
                <ArrowLeft24Regular style={{ fontSize: 14 }} />
                Chat
              </button>
            </div>

            <div className="flex gap-1 mb-2 items-center select-none">
              <div className="flex-1 flex items-center gap-1.5 h-8 px-2 rounded-lg bg-black/30 border border-white/8">
                <Search24Regular className="text-slate-500" style={{ fontSize: 14 }} />
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search…"
                  className="flex-1 bg-transparent text-[11px] text-slate-200 placeholder-slate-500 outline-none"
                />
              </div>
              <button type="button" onClick={handleExportMd} className="ks-icon-btn" title="Copy Markdown">
                <DocumentText24Regular style={{ fontSize: 15 }} />
              </button>
              <button type="button" onClick={handleExportJson} className="ks-icon-btn" title="Copy JSON">
                <Code24Regular style={{ fontSize: 15 }} />
              </button>
              <button type="button" onClick={clearHistory} className="ks-icon-btn hover:text-rose-300" title="Clear history">
                <Delete24Regular style={{ fontSize: 15 }} />
              </button>
            </div>

            <div
              ref={scrollRef}
              onScroll={handleChatScroll}
              className="flex-1 overflow-y-auto px-2 py-2 rounded-xl bg-black/20 border border-white/6 text-slate-300"
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
                  <div className="p-2.5 rounded-xl bg-white/4 border border-white/8">
                    <MarkdownRenderer content={msg.content} />
                  </div>
                </div>
              ))}
            </div>
          </div>
        ) : (
          <div className="flex flex-col flex-1 min-h-0 gap-2">
            {(isRecording || isTranscribing) && (
              <AudioIntelligenceUI
                isRecording={isRecording}
                isTranscribing={isTranscribing}
                captureMode={captureMode}
                usingScreen={usingScreen}
              />
            )}

            <div
              ref={scrollRef}
              onScroll={handleChatScroll}
              className="flex-1 min-h-0 overflow-y-auto px-2 py-2 rounded-xl bg-black/20 border border-white/6 text-slate-300"
            >
              <CapturePreview />

              {sessionHistory.length === 0 && !response && !loading && !error && !screenshotError && (
                <div className="space-y-3 select-none py-1">
                  <div className="rounded-2xl p-3 border border-cyan-400/15 bg-gradient-to-br from-cyan-500/10 to-violet-500/10">
                    <div className="flex items-center gap-2 text-slate-100 font-semibold text-[13px] mb-1">
                      <Sparkle24Filled className="text-cyan-300" style={{ fontSize: 18 }} />
                      Ready when you are
                    </div>
                    <p className="text-[11px] text-slate-400 leading-relaxed">
                      Ask in the box, tap the mic, or capture your screen. Answers stay glanceable.
                    </p>
                    {currentModel && (
                      <p className="text-[10px] text-slate-500 mt-1.5 font-mono truncate">{currentModel}</p>
                    )}
                  </div>

                  <div className="rounded-2xl border border-white/8 bg-white/3 p-2.5 space-y-1.5">
                    <div className="flex items-center gap-1.5 text-[11px] font-semibold text-slate-300 pb-1">
                      <Keyboard24Regular style={{ fontSize: 14 }} className="text-violet-300" />
                      Shortcuts
                    </div>
                    {[
                      ['Screen capture', 'Ctrl+Shift+F'],
                      ['Voice on / off', 'Ctrl+Shift+V'],
                      ['Show / hide', 'Ctrl+Shift+H'],
                    ].map(([label, keys]) => (
                      <div key={keys} className="flex justify-between items-center px-2 py-1.5 rounded-lg bg-black/20">
                        <span className="text-[11px] text-slate-300">{label}</span>
                        <Kbd>{keys}</Kbd>
                      </div>
                    ))}
                    <p className="text-[10px] text-slate-500 text-center pt-1">Double Escape also hides the overlay.</p>
                  </div>
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

              {sessionHistory.map((msg: any) => (
                <div
                  key={msg.id}
                  className={`mb-2.5 p-2.5 rounded-2xl border ${
                    msg.role === 'user'
                      ? 'bg-sky-500/8 border-sky-400/15 ml-4'
                      : 'bg-violet-500/8 border-violet-400/15 mr-2'
                  }`}
                >
                  <div className="flex items-center gap-1.5 text-[10px] font-semibold mb-1.5 text-slate-400">
                    {msg.role === 'user' ? (
                      <Person24Regular className="text-sky-300" style={{ fontSize: 14 }} />
                    ) : (
                      <Bot24Regular className="text-violet-300" style={{ fontSize: 14 }} />
                    )}
                    {msg.role === 'user' ? 'You' : 'KS Vision'}
                    {msg.source ? (
                      <span className="font-medium text-slate-500">· {msg.source}</span>
                    ) : null}
                  </div>
                  <MarkdownRenderer content={msg.content} />
                </div>
              ))}

              {loading && response.length === 0 && !error && !screenshotError && (
                <div ref={answerStartRef} className="flex items-center justify-center py-3">
                  <AIThinking />
                </div>
              )}

              {streaming && response && (
                <div ref={answerStartRef} className="mb-2.5 p-2.5 rounded-2xl bg-cyan-500/10 border border-cyan-400/25">
                  <div className="text-[10px] font-semibold text-cyan-300 mb-1.5 flex items-center gap-1.5">
                    <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse" />
                    Writing…
                  </div>
                  <MarkdownRenderer content={response} />
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      {viewMode === 'chat' && (
        <form onSubmit={handleSubmit} className="flex flex-col gap-1 w-full select-none shrink-0 min-w-0 pt-1">
          <div className="relative w-full">
            <textarea
              value={inputVal}
              onChange={(e) => setInputVal(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={loading ? 'Thinking…' : 'Ask anything…'}
              disabled={loading && !streaming}
              rows={2}
              className="w-full h-[44px] resize-none bg-black/45 border border-white/15 rounded-xl pl-2.5 pr-9 py-1.5 text-[11px] leading-snug text-slate-100 placeholder-slate-500 outline-none focus:border-cyan-400/50 focus:ring-2 focus:ring-cyan-400/20 disabled:opacity-50"
            />
            {loading ? (
              <button
                type="button"
                onClick={cancel}
                className="absolute right-1.5 top-1/2 -translate-y-1/2 h-6 w-6 rounded-lg bg-rose-500/25 border border-rose-400/30 text-rose-200 inline-flex items-center justify-center"
                title="Stop"
              >
                <Dismiss24Regular style={{ fontSize: 12 }} />
              </button>
            ) : (
              <button
                type="submit"
                disabled={!inputVal.trim()}
                className="absolute right-1.5 top-1/2 -translate-y-1/2 h-6 w-6 rounded-lg bg-gradient-to-r from-cyan-500/90 to-violet-500/90 text-white inline-flex items-center justify-center disabled:opacity-30"
                title="Send"
              >
                <ArrowUp24Filled style={{ fontSize: 13 }} />
              </button>
            )}
          </div>

          <ModeToggle mode={captureMode} onChange={setCaptureMode} disabled={isTranscribing} />
        </form>
      )}
    </div>
  );
};
