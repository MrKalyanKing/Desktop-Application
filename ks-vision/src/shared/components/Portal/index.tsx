import React from 'react';
import ReactDOM from 'react-dom';

interface PortalProps {
  children: React.ReactNode;
  containerId?: string;
}

export const Portal: React.FC<PortalProps> = ({ children, containerId }) => {
  const mountNode = containerId ? document.getElementById(containerId) : document.body;
  if (!mountNode) return null;
  return ReactDOM.createPortal(children, mountNode);
};
