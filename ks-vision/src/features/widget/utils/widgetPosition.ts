import type { WidgetPosition } from '../types/widget.types';

export const formatPosition = (pos: WidgetPosition): string => {
  return `X: ${pos.x.toFixed(0)}, Y: ${pos.y.toFixed(0)}`;
};

export const isWithinBounds = (pos: WidgetPosition, bounds: { width: number; height: number }): boolean => {
  return pos.x >= 0 && pos.y >= 0 && pos.x <= bounds.width && pos.y <= bounds.height;
};
