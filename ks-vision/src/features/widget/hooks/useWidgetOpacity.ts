import { useDispatch, useSelector } from 'react-redux';
import { setHovered } from '../stores/widget.store';

// We will use standard select function to bypass typing issues if store is generic
export const useWidgetOpacity = () => {
  const dispatch = useDispatch();
  
  // Selector targets the widget slice
  const opacity = useSelector((state: any) => state.widget.opacity);
  const isHovered = useSelector((state: any) => state.widget.isHovered);

  const handleMouseEnter = () => {
    dispatch(setHovered(true));
  };

  const handleMouseLeave = () => {
    dispatch(setHovered(false));
  };

  return {
    opacity,
    isHovered,
    bind: {
      onMouseEnter: handleMouseEnter,
      onMouseLeave: handleMouseLeave,
    },
  };
};
