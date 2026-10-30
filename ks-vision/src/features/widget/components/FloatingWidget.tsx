import React from 'react';
import { WidgetContainer } from './WidgetContainer';
import { WidgetHeader } from './WidgetHeader';
import { WidgetContent } from './WidgetContent';
import { WidgetOpacity } from './WidgetOpacity';
import { useWidgetOpacity } from '../hooks/useWidgetOpacity';
import { useAutoHide } from '../hooks/useAutoHide';
import '../styles/widget.css';

export const FloatingWidget: React.FC = () => {
  const { opacity, isHovered, bind } = useWidgetOpacity();
  
  // Auto-hide only when widget.autoHideDuration > 0 (min 60s, never while busy)
  useAutoHide(isHovered);

  return (
    <WidgetContainer opacity={opacity} {...bind}>
      <WidgetHeader />
      <WidgetContent />
      <WidgetOpacity opacity={opacity} />
    </WidgetContainer>
  );
};
export default FloatingWidget;
