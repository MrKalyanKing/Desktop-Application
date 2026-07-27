import { createSlice } from '@reduxjs/toolkit';
import type { PayloadAction } from '@reduxjs/toolkit';
import type { WidgetPosition } from '../types/widget.types';
import { OPACITY_DEFAULT } from '../constants/widget.constants';

interface WidgetState {
  visible: boolean;
  opacity: number;
  isHovered: boolean;
  position: WidgetPosition;
}

const initialState: WidgetState = {
  visible: true,
  opacity: OPACITY_DEFAULT,
  isHovered: false,
  position: { x: 0, y: 0 },
};

export const widgetSlice = createSlice({
  name: 'widget',
  initialState,
  reducers: {
    setVisible: (state, action: PayloadAction<boolean>) => {
      state.visible = action.payload;
    },
    toggleVisible: (state) => {
      state.visible = !state.visible;
    },
    setOpacity: (state, action: PayloadAction<number>) => {
      state.opacity = action.payload;
    },
    setHovered: (state, action: PayloadAction<boolean>) => {
      state.isHovered = action.payload;
      state.opacity = action.payload ? 1.0 : OPACITY_DEFAULT;
    },
    setPosition: (state, action: PayloadAction<WidgetPosition>) => {
      state.position = action.payload;
    },
  },
});

export const { setVisible, toggleVisible, setOpacity, setHovered, setPosition } = widgetSlice.actions;
export default widgetSlice.reducer;
