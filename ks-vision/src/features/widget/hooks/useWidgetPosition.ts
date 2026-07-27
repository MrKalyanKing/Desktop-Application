import { useEffect } from 'react';
import { useDispatch, useSelector } from 'react-redux';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { setPosition } from '../stores/widget.store';

export const useWidgetPosition = () => {
  const dispatch = useDispatch();
  const position = useSelector((state: any) => state.widget.position);

  useEffect(() => {
    let active = true;

    const getInitialPosition = async () => {
      try {
        const win = getCurrentWindow();
        const pos = await win.outerPosition();
        if (active) {
          dispatch(setPosition({ x: pos.x, y: pos.y }));
        }
      } catch (err) {
        console.error('Failed to get initial position:', err);
      }
    };

    getInitialPosition();

    let unlisten: (() => void) | undefined;
    const setupListener = async () => {
      try {
        const win = getCurrentWindow();
        unlisten = await win.onMoved(({ payload: pos }) => {
          if (active) {
            dispatch(setPosition({ x: pos.x, y: pos.y }));
          }
        });
      } catch (err) {
        console.error('Failed to listen to window move:', err);
      }
    };

    setupListener();

    return () => {
      active = false;
      if (unlisten) {
        unlisten();
      }
    };
  }, [dispatch]);

  return position;
};
