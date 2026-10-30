import React from 'react';
import { cn } from '../../utils/cn';

export function splitHotkey(value: string): string[] {
  return value
    .split(/[+\s]+/)
    .map((part) => part.trim())
    .filter(Boolean);
}

interface GlassKeyComboProps {
  value: string;
  className?: string;
}

export const GlassKeyCombo: React.FC<GlassKeyComboProps> = ({ value, className }) => {
  const keys = splitHotkey(value);
  if (keys.length === 0) return null;

  return (
    <span className={cn('lg-hotkey-chips inline-flex items-center', className)}>
      {keys.map((key, index) => (
        <React.Fragment key={`${key}-${index}`}>
          {index > 0 && <span className="lg-key-plus">+</span>}
          <kbd className={cn('lg-keycap', index === keys.length - 1 && 'is-last')}>{key}</kbd>
        </React.Fragment>
      ))}
    </span>
  );
};

interface GlassHotkeyFieldProps {
  value: string;
  onChange: (next: string) => void;
}

export const GlassHotkeyField: React.FC<GlassHotkeyFieldProps> = ({ value, onChange }) => {
  return (
    <div className="lg-hotkey-field">
      <GlassKeyCombo value={value} />
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        spellCheck={false}
        aria-label="Shortcut"
      />
    </div>
  );
};

interface GlassToggleProps {
  checked: boolean;
  onChange: (next: boolean) => void;
}

export const GlassToggle: React.FC<GlassToggleProps> = ({ checked, onChange }) => (
  <button
    type="button"
    role="switch"
    aria-checked={checked}
    onClick={() => onChange(!checked)}
    className={cn('lg-toggle', checked && 'is-on')}
  >
    <span className="lg-toggle__knob" />
  </button>
);

interface GlassCheckboxProps {
  checked: boolean;
  onChange: (next: boolean) => void;
}

export const GlassCheckbox: React.FC<GlassCheckboxProps> = ({ checked, onChange }) => (
  <button
    type="button"
    role="checkbox"
    aria-checked={checked}
    onClick={() => onChange(!checked)}
    className={cn('lg-check', checked && 'is-on')}
  >
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
      <path d="M2.5 6.2L5 8.7L9.5 3.5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  </button>
);
