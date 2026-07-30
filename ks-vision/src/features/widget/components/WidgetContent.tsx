import React, { useEffect, useState, useRef } from 'react';
import { CpuIcon } from '../../../shared/components/Icon';
import { useWidgetPosition } from '../hooks/useWidgetPosition';
import { formatPosition } from '../utils/widgetPosition';
import { AIStatus, AIThinking, useAI } from '../../ai';
import { CapturePreview, useScreenshot } from '../../screenshot';
import { useSystemTray } from '../../tray';
import { SettingsPanel } from '../../settings';
import { useVoiceAgent } from '../hooks/useVoiceAgent';
import { ModeToggle } from './ModeToggle';
import { AudioIntelligenceUI } from './AudioIntelligenceUI';

export const WidgetContent: React.FC = () => {
  const position = useWidgetPosition();
  const [cpuUsage, setCpuUsage] = useState(12);
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

  const { isRecording, isTranscribing, captureMode, setCaptureMode, toggleVoice } = useVoiceAgent();

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

  useEffect(() => {
    // Start freshly: do NOT load database history on mount!
  }, []);

  useEffect(() => {
    const interval = setInterval(() => {
      setCpuUsage((prev) => {
        const delta = Math.floor(Math.random() * 7) - 3;
        const next = prev + delta;
        return Math.max(5, Math.min(45, next));
      });
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleChatScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const distFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    userScrolledAwayRef.current = distFromBottom > 64;
  };

  // When a new answer starts: show the start of that answer once — never chase the end while streaming.
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

    // Do not auto-jump during token streaming (`response` updates).
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

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      handleSubmit(e);
    }
  };

  const handleExportMd = async () => {
    const data = await exportHistoryMarkdown();
    if (data) {
      navigator.clipboard.writeText(data).catch(() => {});
    }
  };

  const handleExportJson = async () => {
    const data = await exportHistoryJson();
    if (data) {
      navigator.clipboard.writeText(data).catch(() => {});
    }
  };

  const getStatusBadge = () => {
    if (isRecording) {
      return (
        <span className="text-[8px] font-bold text-red-400 bg-red-950/20 px-1.5 py-0.5 rounded border border-red-800/20 animate-pulse select-none">
          🎤 RECORDING
        </span>
      );
    }
    switch (screenshotStep) {
      case 'capturing':
        return (
          <span className="text-[8px] font-bold text-yellow-400 bg-yellow-950/20 px-1.5 py-0.5 rounded border border-yellow-800/20 animate-pulse select-none">
            📸 CAPTURING
          </span>
        );
      case 'ocr':
        return (
          <span className="text-[8px] font-bold text-orange-400 bg-orange-950/20 px-1.5 py-0.5 rounded border border-orange-800/20 animate-pulse select-none">
            🔍 EXTRACTING
          </span>
        );
      case 'ai':
        return (
          <span className="text-[8px] font-bold text-cyan-400 bg-cyan-950/20 px-1.5 py-0.5 rounded border border-cyan-800/20 animate-pulse select-none">
            🤖 ANALYSING
          </span>
        );
      case 'done':
        return (
          <span className="text-[8px] font-bold text-emerald-400 bg-emerald-950/20 px-1.5 py-0.5 rounded border border-emerald-800/20 select-none">
            ✅ READY
          </span>
        );
      case 'error':
        return (
          <span className="text-[8px] font-bold text-rose-400 bg-rose-950/20 px-1.5 py-0.5 rounded border border-rose-800/20 select-none">
            ❌ ERROR
          </span>
        );
      default:
        return null;
    }
  };

  const showAIPanel = true;

  const filteredHistory = conversationHistory.filter((msg: any) =>
    msg.content.toLowerCase().includes(searchQuery.toLowerCase())
  );

  if (showSettings) {
    return (
      <div className="flex-1 p-2 flex flex-col justify-between text-xs bg-slate-900/10 min-h-0">
        <SettingsPanel onClose={() => setShowSettings(false)} />
      </div>
    );
  }

  return (
    <div className="flex-1 p-2 flex flex-col justify-between text-xs bg-slate-900/10 min-h-0">
      {/* Top row: AI Status, Context badges, and Position coordinates */}
      <div className="flex items-center justify-between mb-1 select-none">
        <div className="flex items-center gap-1.5">
          <AIStatus />
          {getStatusBadge()}
          {/* Active Context Badges */}
          {activeSources.map((src: string) => (
            <span key={src} className="text-[8px] font-bold text-cyan-400 bg-cyan-950/20 px-1.5 py-0.5 rounded border border-cyan-800/10 animate-pulse select-none">
              {src === 'Voice' ? '🎤 Voice' : src === 'OCR' ? '📄 OCR' : src}
            </span>
          ))}
          {showAIPanel && !screenshotStep && activeSources.length === 0 && !isRecording && (
            <span className="text-[8px] font-mono text-slate-500 bg-slate-950/20 px-1 py-0.5 rounded border border-slate-800/10 max-w-[80px] truncate">
              {currentModel}
            </span>
          )}
        </div>
        <span className="text-[9px] font-mono text-cyan-400/80 bg-cyan-950/20 px-1.5 py-0.5 rounded border border-cyan-800/10">
          {formatPosition(position)}
        </span>
      </div>

      {/* Center content */}
      <div className="flex-1 min-h-0 flex flex-col justify-center my-0.5">
        <div className="flex-1 flex flex-col min-h-0">
          
          {viewMode === 'history' ? (
            /* ========================================================
               HISTORY VIEW: Complete read-only SQLite database logs
               ======================================================== */
            <div className="flex flex-col flex-1 min-h-0">
              {/* Header and Back Button */}
              <div className="flex justify-between items-center mb-1.5 border-b border-slate-850/60 pb-1.5 select-none">
                <span className="text-[10px] font-bold text-cyan-400 tracking-wider">📜 DATABASE LOGS</span>
                <button
                  type="button"
                  onClick={() => setViewMode('chat')}
                  className="h-5 px-2 bg-slate-950 border border-slate-800 hover:bg-slate-900 text-slate-400 hover:text-cyan-400 text-[8.5px] font-bold rounded cursor-pointer transition-all active:scale-95"
                >
                  ← Back to Chat
                </button>
              </div>

              {/* Search, Export, and Clear DB Actions */}
              <div className="flex gap-1 mb-1.5 items-center select-none">
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="Search database logs..."
                  className="flex-1 h-5 bg-slate-950/60 border border-slate-800/40 rounded px-1.5 py-0.5 text-[9px] text-slate-300 placeholder-slate-500 outline-none focus:border-cyan-500/30"
                />
                <button
                  type="button"
                  onClick={handleExportMd}
                  className="h-5 px-1.5 bg-slate-950/60 border border-slate-800/40 hover:bg-cyan-950/20 text-slate-450 hover:text-cyan-400 text-[8px] font-bold rounded cursor-pointer"
                  title="Export Markdown"
                >
                  MD
                </button>
                <button
                  type="button"
                  onClick={handleExportJson}
                  className="h-5 px-1.5 bg-slate-950/60 border border-slate-800/40 hover:bg-cyan-950/20 text-slate-450 hover:text-cyan-400 text-[8px] font-bold rounded cursor-pointer"
                  title="Export JSON"
                >
                  JSON
                </button>
                <button
                  type="button"
                  onClick={clearHistory}
                  className="h-5 px-1.5 bg-rose-950/40 border border-rose-900/40 hover:bg-rose-900/60 text-rose-300 text-[8px] font-bold rounded cursor-pointer transition-all active:scale-95"
                  title="Clear all database history"
                >
                  🗑️ Clear
                </button>
              </div>

              {/* Scroll Container showing Database Logs */}
              <div 
                ref={scrollRef}
                onScroll={handleChatScroll}
                className="flex-1 overflow-y-auto px-1.5 py-1 bg-slate-950/40 border border-slate-800/40 rounded-lg text-xs leading-relaxed text-slate-300 font-medium select-text"
              >
                {filteredHistory.length === 0 && (
                  <div className="text-center py-6 text-slate-500 text-[9px] select-none">
                    No database history logs found.
                  </div>
                )}
                {filteredHistory.map((msg: any) => (
                  <div key={msg.id} className="mb-2 p-1.5 bg-slate-900/30 border border-slate-850/40 rounded-md relative group select-text">
                    <div className="flex justify-between items-center text-[7.5px] font-bold text-slate-500 mb-0.5 select-none">
                      <span>{msg.role === 'user' ? 'USER' : 'COPAILOT'} {msg.source ? `[${msg.source}]` : ''}</span>
                      <button
                        type="button"
                        onClick={() => msg.id && deleteMessage(msg.id)}
                        className="opacity-0 group-hover:opacity-100 hover:text-rose-400 transition-opacity text-[8px] cursor-pointer"
                        title="Delete this message"
                      >
                        ✕
                      </button>
                    </div>
                    <div className="whitespace-pre-wrap font-sans text-slate-300 text-xs">
                      {msg.content}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : (
            /* ========================================================
               CHAT VIEW: Session-only live messaging (starts empty)
               ======================================================== */
            <div className="flex flex-col flex-1 min-h-0">
              {/* Audio Intelligence UI Dashboard */}
              {(isRecording || isTranscribing) && (
                <AudioIntelligenceUI
                  isRecording={isRecording}
                  isTranscribing={isTranscribing}
                  captureMode={captureMode}
                />
              )}

              {/* Chat Scroll Container showing Session messages */}
              <div 
                ref={scrollRef}
                onScroll={handleChatScroll}
                className="flex-1 overflow-y-auto px-1.5 py-1 bg-slate-950/40 border border-slate-800/40 rounded-lg text-xs leading-relaxed text-slate-300 font-medium select-text"
              >
                <CapturePreview />

                {/* If session history is empty and no active response, render the CPU bar and the Quick Guide */}
                {sessionHistory.length === 0 && !response && !loading && !error && !screenshotError && (
                  <div className="space-y-3 select-none">
                    {/* CPU Bar */}
                    <div className="flex items-center gap-3 bg-slate-950/20 p-2 rounded-lg border border-slate-800/10 my-0.5">
                      <CpuIcon size={14} className="text-purple-450/80 animate-pulse" />
                      <div className="flex-1 flex flex-col gap-1">
                        <div className="flex items-center justify-between text-[9px] font-bold text-slate-400">
                          <span>CPU AGENT WORKLOAD</span>
                          <span className="font-mono text-slate-200">{cpuUsage}%</span>
                        </div>
                        <div className="h-1.5 w-full bg-slate-800/60 rounded-full overflow-hidden">
                          <div 
                            className="h-full bg-gradient-to-r from-cyan-400 to-purple-400 rounded-full transition-all duration-500" 
                            style={{ width: `${cpuUsage}%` }}
                          />
                        </div>
                      </div>
                    </div>

                    {/* Quick Guide Card */}
                    <div className="bg-slate-900/25 border border-slate-850/40 rounded-lg p-2.5 space-y-2">
                      <div className="text-[8.5px] font-bold text-cyan-450 tracking-wider uppercase border-b border-slate-800/60 pb-1">
                        🚀 Quick Action Shortcuts
                      </div>
                      <div className="grid grid-cols-1 gap-1.5 text-[9px]">
                        <div className="flex justify-between items-center bg-slate-950/20 px-2 py-1 rounded border border-slate-800/10">
                          <span className="text-slate-300">✂️ Snipping Tool (Region Selection)</span>
                          <kbd className="font-mono bg-slate-950/60 text-cyan-300 border border-slate-800 px-1 py-0.2 rounded text-[8px] font-bold">Ctrl+Shift+R</kbd>
                        </div>
                        <div className="flex justify-between items-center bg-slate-950/20 px-2 py-1 rounded border border-slate-800/10">
                          <span className="text-slate-300">🖥️ Capture Full Screen</span>
                          <kbd className="font-mono bg-slate-950/60 text-cyan-300 border border-slate-800 px-1 py-0.2 rounded text-[8px] font-bold">Ctrl+Shift+F</kbd>
                        </div>
                        <div className="flex justify-between items-center bg-slate-950/20 px-2 py-1 rounded border border-slate-800/10">
                          <span className="text-slate-300">🪟 Capture Active Window</span>
                          <kbd className="font-mono bg-slate-950/60 text-cyan-300 border border-slate-800 px-1 py-0.2 rounded text-[8px] font-bold">Ctrl+Shift+W</kbd>
                        </div>
                        <div className="flex justify-between items-center bg-slate-950/20 px-2 py-1 rounded border border-slate-800/10">
                          <span className="text-slate-300">🎤 Toggle Ambient Voice Input</span>
                          <kbd className="font-mono bg-slate-950/60 text-cyan-300 border border-slate-800 px-1 py-0.2 rounded text-[8px] font-bold">Ctrl+Shift+V</kbd>
                        </div>
                        <div className="flex justify-between items-center bg-slate-950/20 px-2 py-1 rounded border border-slate-800/10">
                          <span className="text-slate-300">👁️ Show / Hide Widget Window</span>
                          <kbd className="font-mono bg-slate-950/60 text-cyan-300 border border-slate-800 px-1 py-0.2 rounded text-[8px] font-bold">Ctrl+Shift+H</kbd>
                        </div>
                      </div>
                      <div className="text-[7.5px] text-slate-500 text-center pt-1 italic">
                        Tip: Pressing "Escape" twice quickly hides the widget window.
                      </div>
                    </div>
                  </div>
                )}

                {(error || screenshotError) && (
                  <div className="p-1.5 rounded bg-rose-950/30 border border-rose-800/30 text-rose-300 text-[9px] mt-1 select-text">
                    <div className="font-bold mb-0.5">ERROR: {error?.type || 'SCREENSHOT_ERROR'}</div>
                    {error?.message || screenshotError}
                  </div>
                )}

                {/* Render dynamic session conversation history logs */}
                {sessionHistory.map((msg: any) => (
                  <div key={msg.id} className="mb-2 p-1.5 bg-slate-900/30 border border-slate-850/40 rounded-md relative group select-text">
                    <div className="flex justify-between items-center text-[7.5px] font-bold text-slate-500 mb-0.5 select-none">
                      <span>{msg.role === 'user' ? 'USER' : 'COPAILOT'} {msg.source ? `[${msg.source}]` : ''}</span>
                    </div>
                    <div className="whitespace-pre-wrap font-sans text-slate-300 text-xs">
                      {msg.content}
                    </div>
                  </div>
                ))}
                
                {loading && response.length === 0 && !error && !screenshotError && (
                  <div ref={answerStartRef} className="flex items-center justify-center py-2 select-none animate-pulse">
                    <AIThinking />
                  </div>
                )}
                
                {streaming && response && (
                  <div ref={answerStartRef} className="mb-2 p-1.5 bg-cyan-950/10 border border-cyan-900/20 rounded-md select-text">
                    <div className="text-[7.5px] font-bold text-cyan-400 mb-0.5 select-none">
                      COPAILOT (streaming...)
                    </div>
                    <div className="whitespace-pre-wrap font-sans text-cyan-200 text-xs">
                      {response}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

        </div>
      </div>

      {/* Audio Capture Mode Toggle */}
      {viewMode === 'chat' && (
        <div className="mb-1">
          <ModeToggle mode={captureMode} onChange={setCaptureMode} disabled={isTranscribing} />
        </div>
      )}

      {/* Bottom row */}
      {viewMode === 'chat' && (
        <form onSubmit={handleSubmit} className="flex gap-1 items-center mt-1 select-none">
          <input 
            type="text"
            value={inputVal}
            onChange={(e) => setInputVal(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={loading ? "Generating response..." : "Ask Copilot..."}
            disabled={loading && !streaming}
            className="flex-1 h-6 bg-slate-950/50 border border-slate-800/60 rounded-md px-2 py-0.5 text-[10px] text-slate-100 placeholder-slate-500 outline-none focus:border-cyan-500/40 disabled:opacity-50 transition-all min-w-0"
          />

          {/* Toggle Voice Recording */}
          <button
            type="button"
            onClick={toggleVoice}
            className={`h-6 w-6 flex items-center justify-center border rounded-md text-[9px] font-bold transition-all active:scale-95 cursor-pointer ${
              isRecording 
                ? 'bg-red-950 border-red-800/40 text-red-400 animate-pulse' 
                : 'bg-slate-950/40 border-slate-800/40 text-slate-400 hover:text-cyan-450 hover:border-cyan-800/40'
            }`}
            title={
              isRecording
                ? 'Stop voice listening'
                : `Start voice listening (${captureMode})`
            }
          >
            🎤
          </button>

          {/* Toggle Settings Gear */}
          <button
            type="button"
            onClick={() => setShowSettings(!showSettings)}
            className={`h-6 w-6 flex items-center justify-center border rounded-md text-[9px] font-bold transition-all active:scale-95 cursor-pointer ${
              showSettings 
                ? 'bg-cyan-950 border-cyan-800/40 text-cyan-300' 
                : 'bg-slate-950/40 border-slate-800/40 text-slate-400'
            }`}
            title="Open preferences"
          >
            ⚙️
          </button>

          {/* Toggle Auto Copy */}
          <button
            type="button"
            onClick={() => setAutoCopy(!autoCopyClipboard)}
            className={`h-6 w-6 flex items-center justify-center border rounded-md text-[9px] font-bold transition-all active:scale-95 cursor-pointer ${
              autoCopyClipboard 
                ? 'bg-cyan-950/40 border-cyan-800/40 text-cyan-300' 
                : 'bg-slate-950/40 border-slate-800/40 text-slate-500'
            }`}
            title={autoCopyClipboard ? "Auto-copy active" : "Auto-copy disabled"}
          >
            📋
          </button>

          {/* Toggle History Tab */}
          <button
            type="button"
            onClick={() => {
              setViewMode('history');
              loadHistory();
            }}
            className="h-6 w-6 flex items-center justify-center bg-slate-950/40 border border-slate-800/40 hover:bg-cyan-950/20 hover:border-cyan-850 text-slate-400 hover:text-cyan-400 text-[9px] font-bold rounded-md cursor-pointer transition-all active:scale-95"
            title="Open database history logs"
          >
            📜
          </button>
          
          {loading ? (
            <button
              type="button"
              onClick={cancel}
              className="h-6 px-2 bg-rose-950/40 border border-rose-800/40 hover:bg-rose-900/40 text-rose-300 text-[9px] font-bold rounded-md cursor-pointer transition-all active:scale-95 whitespace-nowrap"
            >
              Cancel
            </button>
          ) : (
            <button
              type="submit"
              disabled={!inputVal.trim()}
              className="h-6 px-2.5 bg-cyan-950/40 border border-cyan-800/40 hover:bg-cyan-900/40 text-cyan-300 text-[9px] font-bold rounded-md cursor-pointer disabled:opacity-30 transition-all active:scale-95 disabled:pointer-events-none"
            >
              Send
            </button>
          )}
        </form>
      )}
    </div>
  );
};
