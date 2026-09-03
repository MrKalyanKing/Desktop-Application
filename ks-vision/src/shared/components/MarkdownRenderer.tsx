import React, { useState } from 'react';

interface MarkdownRendererProps {
  content: string;
}

/**
 * Deduplicates repeated full text blocks if model output accidentally doubles.
 */
function deduplicateText(text: string): string {
  const trimmed = text.trim();
  if (trimmed.length > 50) {
    const half = Math.floor(trimmed.length / 2);
    const firstHalf = trimmed.slice(0, half).trim();
    const secondHalf = trimmed.slice(half).trim();
    if (secondHalf.startsWith(firstHalf)) {
      return secondHalf;
    }
  }
  return text;
}

/**
 * ChatGPT & Claude style Markdown & Code block renderer.
 * Renders headers, lists, code blocks with copy buttons, bold text, and checklists.
 */
export const MarkdownRenderer: React.FC<MarkdownRendererProps> = ({ content }) => {
  const cleanContent = deduplicateText(content);

  // Split content into code blocks and normal markdown segments
  const parts = cleanContent.split(/(```[\s\S]*?```)/g);

  return (
    <div className="space-y-2.5 text-xs text-slate-200 leading-relaxed select-text font-sans">
      {parts.map((part, index) => {
        if (part.startsWith('```') && part.endsWith('```')) {
          // Code block rendering
          const raw = part.slice(3, -3);
          const firstLineEnd = raw.indexOf('\n');
          let language = 'code';
          let codeText = raw;

          if (firstLineEnd !== -1) {
            const possibleLang = raw.slice(0, firstLineEnd).trim();
            if (possibleLang && !possibleLang.includes(' ')) {
              language = possibleLang;
              codeText = raw.slice(firstLineEnd + 1);
            }
          }

          return <CodeBlock key={index} code={codeText.trim()} language={language} />;
        }

        // Standard text / Markdown block rendering
        return <TextBlock key={index} text={part} />;
      })}
    </div>
  );
};

const CodeBlock: React.FC<{ code: string; language: string }> = ({ code, language }) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="my-2.5 rounded-xl border border-slate-800 bg-slate-950 shadow-lg overflow-hidden group">
      {/* Code Header Bar */}
      <div className="flex items-center justify-between px-3 py-1.5 bg-slate-900/90 border-b border-slate-800 text-[10px] font-mono text-slate-400 select-none">
        <span className="font-semibold text-cyan-400 tracking-wide uppercase">{language}</span>
        <button
          type="button"
          onClick={handleCopy}
          className="flex items-center gap-1.5 px-2 py-0.5 rounded bg-slate-800/60 hover:bg-cyan-950/40 hover:text-cyan-300 border border-slate-700/50 transition-all cursor-pointer text-[9.5px]"
        >
          {copied ? (
            <span className="text-emerald-400 font-bold">✓ Copied</span>
          ) : (
            <span>📋 Copy code</span>
          )}
        </button>
      </div>

      {/* Code Body */}
      <pre className="p-3 overflow-x-auto text-[11px] font-mono text-slate-200 leading-relaxed bg-slate-950/90 selection:bg-cyan-900/50">
        <code>{code}</code>
      </pre>
    </div>
  );
};

const TextBlock: React.FC<{ text: string }> = ({ text }) => {
  const lines = text.split('\n');

  return (
    <div className="space-y-1.5">
      {lines.map((line, idx) => {
        const trimmed = line.trim();
        if (!trimmed) return <div key={idx} className="h-1" />;

        // Headers
        if (line.startsWith('# ')) {
          return (
            <h1 key={idx} className="text-sm font-bold text-cyan-300 border-b border-slate-800 pb-1 mt-2 mb-1">
              {formatInline(line.slice(2))}
            </h1>
          );
        }
        if (line.startsWith('## ')) {
          return (
            <h2 key={idx} className="text-xs font-bold text-purple-300 mt-2 mb-1">
              {formatInline(line.slice(3))}
            </h2>
          );
        }
        if (line.startsWith('### ')) {
          return (
            <h3 key={idx} className="text-xs font-semibold text-cyan-400 mt-1.5 mb-0.5">
              {formatInline(line.slice(4))}
            </h3>
          );
        }

        // Bullet lists (* or -)
        if (trimmed.startsWith('* ') || trimmed.startsWith('- ')) {
          return (
            <div key={idx} className="flex gap-2 items-start pl-2 text-slate-300">
              <span className="text-cyan-400 font-bold text-[10px] mt-0.5">•</span>
              <span className="flex-1">{formatInline(trimmed.slice(2))}</span>
            </div>
          );
        }

        // Numbered lists / Checklists (1. 2. 3.)
        const matchNum = trimmed.match(/^(\d+\.)\s+(.*)/);
        if (matchNum) {
          return (
            <div key={idx} className="flex gap-2 items-start pl-2 text-slate-300">
              <span className="font-mono text-purple-400 font-semibold text-[11px]">{matchNum[1]}</span>
              <span className="flex-1">{formatInline(matchNum[2])}</span>
            </div>
          );
        }

        // Standard paragraph
        return (
          <p key={idx} className="text-slate-200 leading-relaxed">
            {formatInline(line)}
          </p>
        );
      })}
    </div>
  );
};

/**
 * Format inline elements like **bold**, *italic*, and `inline code`.
 */
function formatInline(str: string): React.ReactNode[] {
  const tokens = str.split(/(\*\*.*?\*\*|`.*?`|\*.*?\*)/g);

  return tokens.map((token, index) => {
    if (token.startsWith('**') && token.endsWith('**')) {
      return (
        <strong key={index} className="font-bold text-slate-100">
          {token.slice(2, -2)}
        </strong>
      );
    }
    if (token.startsWith('`') && token.endsWith('`')) {
      return (
        <code
          key={index}
          className="px-1.5 py-0.5 rounded bg-slate-900 border border-slate-800 font-mono text-[10.5px] text-cyan-300"
        >
          {token.slice(1, -1)}
        </code>
      );
    }
    if (token.startsWith('*') && token.endsWith('*')) {
      return (
        <em key={index} className="italic text-slate-300">
          {token.slice(1, -1)}
        </em>
      );
    }
    return token;
  });
}
