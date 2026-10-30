import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';

export type OpKind = 'typed' | 'capture' | 'scroll' | 'voice';
export type NoticeLevel = 'info' | 'success' | 'error';

/**
 * Mirror of the Rust OpRegistry's cancelled set. F3 cancels in Rust and broadcasts
 * `ks://cancelled`; every completion path checks here before touching UI, history,
 * clipboard or the F4 answer cache.
 */
const CANCELLED_CAP = 256;
const cancelled = new Set<number>();
let listening: Promise<() => void> | null = null;

const remember = (ids: number[]) => {
  for (const id of ids) cancelled.add(id);
  while (cancelled.size > CANCELLED_CAP) {
    const oldest = cancelled.values().next().value as number;
    cancelled.delete(oldest);
  }
};

export const opsService = {
  /** Install the `ks://cancelled` listener once per window. */
  init: () => {
    if (!listening) {
      listening = listen<{ ids: number[] }>('ks://cancelled', (e) => remember(e.payload?.ids ?? []));
    }
    return listening;
  },

  begin: (kind: OpKind) => invoke<number>('begin_op_cmd', { kind }),

  end: (id: number | null | undefined) => {
    if (id == null) return;
    invoke('end_op_cmd', { id }).catch(() => {});
  },

  isCancelled: (id: number | null | undefined) => id != null && cancelled.has(id),

  /** F4 cache; Rust ignores cancelled ids and answers older than the stored one. */
  setLastAnswer: (id: number | null | undefined, text: string) => {
    if (id == null || cancelled.has(id) || !text.trim()) return;
    invoke('set_last_answer_cmd', { opId: id, text }).catch(() => {});
  },
};

export const showNotice = (level: NoticeLevel, message: string) => {
  emit('ks://notice', { level, message }).catch(() => {});
};
