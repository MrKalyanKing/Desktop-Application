import React from 'react';
import { cn } from '../../../shared/utils/cn';

interface WidgetContainerProps extends React.HTMLAttributes<HTMLDivElement> {
  children: React.ReactNode;
  opacity: number;
}

export const WidgetContainer: React.FC<WidgetContainerProps> = ({
  children,
  opacity,
  className,
  ...props
}) => {
  return (
    <div className={cn('lg-panel', className)} {...props}>
      {/* Transparency applies to the background only; text always stays fully opaque. */}
      <div
        className="lg-panel__material"
        aria-hidden="true"
        style={{ opacity, transition: 'opacity 0.35s cubic-bezier(0.22, 1, 0.36, 1)' }}
      >
        <div className="lg-panel__blur" />
        <div className="lg-panel__tint" />
        <div className="lg-panel__orbs">
          <span className="lg-orb lg-orb--a" />
          <span className="lg-orb lg-orb--b" />
          <span className="lg-orb lg-orb--c" />
        </div>
        <div className="lg-panel__noise" />
        <div className="lg-panel__sheen" />
        <div className="lg-panel__rim" />
      </div>
      <div className="lg-panel__content">{children}</div>
    </div>
  );
};
