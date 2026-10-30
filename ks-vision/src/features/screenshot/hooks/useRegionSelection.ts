import { useState } from 'react';

export interface SelectionRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

const STEP = 20;
const FINE_STEP = 2; // Ctrl
const MIN_SIZE = 10;

const clampRect = (r: SelectionRect): SelectionRect => {
  const vw = window.innerWidth;
  const vh = window.innerHeight;
  const width = Math.min(Math.max(r.width, MIN_SIZE), vw);
  const height = Math.min(Math.max(r.height, MIN_SIZE), vh);
  return {
    width,
    height,
    x: Math.min(Math.max(r.x, 0), vw - width),
    y: Math.min(Math.max(r.y, 0), vh - height),
  };
};

/** Centred rectangle covering ~40% of the screen area. */
const initialRect = (): SelectionRect => {
  const width = Math.round(window.innerWidth * 0.63);
  const height = Math.round(window.innerHeight * 0.63);
  return clampRect({
    x: Math.round((window.innerWidth - width) / 2),
    y: Math.round((window.innerHeight - height) / 2),
    width,
    height,
  });
};

/**
 * Region selection in CSS pixels of the selector window. Keyboard: arrows move,
 * Shift+arrows resize, Ctrl switches either to a 2px step. Mouse drag still works.
 */
export const useRegionSelection = () => {
  const [rect, setRect] = useState<SelectionRect>(initialRect);
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null);

  const handleMouseDown = (e: React.MouseEvent) => {
    setDragStart({ x: e.clientX, y: e.clientY });
    setRect({ x: e.clientX, y: e.clientY, width: 0, height: 0 });
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!dragStart) return;
    setRect({
      x: Math.min(dragStart.x, e.clientX),
      y: Math.min(dragStart.y, e.clientY),
      width: Math.abs(dragStart.x - e.clientX),
      height: Math.abs(dragStart.y - e.clientY),
    });
  };

  const handleMouseUp = () => {
    setDragStart(null);
  };

  /** Returns true when the key adjusted the selection. */
  const handleArrowKey = (e: KeyboardEvent): boolean => {
    const dir: Record<string, [number, number]> = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    };
    const d = dir[e.key];
    if (!d) return false;
    const step = e.ctrlKey ? FINE_STEP : STEP;
    setRect((r) =>
      clampRect(
        e.shiftKey
          ? { ...r, width: r.width + d[0] * step, height: r.height + d[1] * step }
          : { ...r, x: r.x + d[0] * step, y: r.y + d[1] * step }
      )
    );
    return true;
  };

  return {
    isSelecting: dragStart !== null,
    selectionRect: rect,
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
    handleArrowKey,
  };
};
