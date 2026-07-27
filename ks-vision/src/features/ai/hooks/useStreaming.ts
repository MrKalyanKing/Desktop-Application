import { useDispatch } from 'react-redux';
import { appendLastResponse } from '../stores/ai.store';

export const useStreaming = () => {
  const dispatch = useDispatch();

  const handleChunk = (chunk: string) => {
    dispatch(appendLastResponse(chunk));
  };

  return {
    handleChunk,
  };
};
