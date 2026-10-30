import React, { useLayoutEffect, useMemo, useRef, useState } from 'react';
import {
  CheckmarkCircle24Filled,
  ChevronDown24Regular,
  ChevronUp24Regular,
  Clipboard24Regular,
  Mic24Filled,
  Person24Regular,
} from '@fluentui/react-icons';
import { MarkdownRenderer } from '../../../shared/components/MarkdownRenderer';
import type { ChatMessage } from '../../ai/types/ai.types';

/** One question and its answer. */
export interface Exchange {
  key: string;
  question?: string;
  answer?: string;
  /** 'voice' | 'system' | 'chat' … */
  source?: string;
  latencyMs?: number;
}

/** Pair session messages into question → answer exchanges. */
export const toExchanges = (messages: ChatMessage[]): Exchange[] => {
  const out: Exchange[] = [];
  for (const m of messages) {
    if (m.role === 'user') {
      out.push({ key: `q${m.id}`, question: m.content, source: m.source || 'chat' });
    } else if (m.role === 'assistant') {
      const last = out[out.length - 1];
      if (last && last.answer === undefined && !m.question) {
        last.answer = m.content;
        last.latencyMs = m.latencyMs;
      } else {
        out.push({
          key: `a${m.id}`,
          question: m.question,
          answer: m.content,
          source: m.source,
          latencyMs: m.latencyMs,
        });
      }
    }
  }
  return out;
};

const CopyButton: React.FC<{ text: string }> = ({ text }) => {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      className="ks-qa__copy"
      title={copied ? 'Copied' : 'Copy answer (F4)'}
      onClick={() => {
        navigator.clipboard.writeText(text).catch(() => {});
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1500);
      }}
    >
      {copied ? <CheckmarkCircle24Filled style={{ fontSize: 14 }} /> : <Clipboard24Regular style={{ fontSize: 14 }} />}
      {copied ? 'Copied' : 'Copy'}
    </button>
  );
};

/** While streaming, close an unfinished ``` fence so the partial code renders as code. */
const renderable = (text: string, live?: boolean) =>
  live && (text.match(/```/g)?.length ?? 0) % 2 === 1 ? `${text}\n\`\`\`` : text;

const sourceLabel = (source?: string) =>
  source === 'system' ? 'Meeting' : source === 'voice' ? 'You said' : 'You asked';

const QACard: React.FC<{
  exchange: Exchange;
  live?: boolean;
  thinking?: boolean;
  compact?: boolean;
}> = ({ exchange, live, thinking, compact }) => {
  const { question, answer, source, latencyMs } = exchange;
  const isVoice = source === 'voice' || source === 'system';
  return (
    <article className={`ks-qa ${compact ? 'ks-qa--compact' : ''} ${live ? 'is-live' : ''}`}>
      <header className="ks-qa__question">
        <span className="ks-qa__label">
          {isVoice ? <Mic24Filled style={{ fontSize: 11 }} /> : <Person24Regular style={{ fontSize: 11 }} />}
          {sourceLabel(source)}
          {typeof latencyMs === 'number' && !compact && (
            <span className="ks-qa__latency">· answered in {(latencyMs / 1000).toFixed(1)}s</span>
          )}
        </span>
        <h2 className="ks-qa__question-text">{question || (live ? 'Listening to the question…' : 'Question')}</h2>
      </header>

      <div className="ks-qa__answer">
        {thinking && !answer ? (
          <div className="ks-qa__thinking" role="status">
            <span />
            <span />
            <span />
            Thinking…
          </div>
        ) : (
          <MarkdownRenderer content={renderable(answer || '', live)} />
        )}
        {live && answer && <span className="ks-qa__caret" aria-hidden />}
      </div>

      {!live && answer && !compact && (
        <footer className="ks-qa__footer">
          <CopyButton text={answer} />
        </footer>
      )}
    </article>
  );
};

interface AnswerViewProps {
  exchanges: Exchange[];
  /** The exchange being answered right now, if any. */
  live: Exchange | null;
  thinking: boolean;
}

/**
 * Focus view: only the current question and answer, large and on a solid card.
 * Earlier answers stay folded away so they never get mistaken for the new one.
 */
export const AnswerView: React.FC<AnswerViewProps> = ({ exchanges, live, thinking }) => {
  const [showEarlier, setShowEarlier] = useState(false);
  const currentRef = useRef<HTMLDivElement>(null);

  const current = live ?? exchanges[exchanges.length - 1] ?? null;
  const earlier = useMemo(
    () => (live ? exchanges : exchanges.slice(0, -1)).slice().reverse(),
    [exchanges, live]
  );

  // New question: bring its start into view and fold the history away.
  const currentKey = current?.key;
  useLayoutEffect(() => {
    if (!currentKey) return;
    setShowEarlier(false);
    currentRef.current?.scrollIntoView({ block: 'start' });
  }, [currentKey]);

  if (!current) return null;

  return (
    <div className="ks-answers">
      <div ref={currentRef}>
        <QACard exchange={current} live={!!live} thinking={thinking} />
      </div>

      {earlier.length > 0 && (
        <div className="ks-earlier">
          <button type="button" className="ks-earlier__toggle" onClick={() => setShowEarlier((v) => !v)}>
            {showEarlier ? <ChevronUp24Regular style={{ fontSize: 14 }} /> : <ChevronDown24Regular style={{ fontSize: 14 }} />}
            {showEarlier ? 'Hide earlier answers' : `Earlier answers (${earlier.length})`}
          </button>
          {showEarlier && earlier.map((ex) => <QACard key={ex.key} exchange={ex} compact />)}
        </div>
      )}
    </div>
  );
};
