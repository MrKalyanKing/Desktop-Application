import type { KeyboardShortcut } from '../types/hotkey.types';

export const isHotkeyMatched = (e: KeyboardEvent, shortcut: KeyboardShortcut): boolean => {
  return (
    e.key.toLowerCase() === shortcut.key.toLowerCase() &&
    !!e.ctrlKey === !!shortcut.ctrlKey &&
    !!e.shiftKey === !!shortcut.shiftKey &&
    !!e.altKey === !!shortcut.altKey &&
    !!e.metaKey === !!shortcut.metaKey
  );
};
