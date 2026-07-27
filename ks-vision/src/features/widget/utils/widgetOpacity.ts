export const getOpacityClass = (opacity: number): string => {
  if (opacity <= 0.25) return 'opacity-25';
  if (opacity >= 1.0) return 'opacity-100';
  return `opacity-[${opacity}]`;
};

export const getWidgetStyle = (opacity: number): React.CSSProperties => {
  return {
    opacity,
    transition: 'opacity 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
  };
};
