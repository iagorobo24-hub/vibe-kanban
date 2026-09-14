import * as React from 'react';
import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';

type FieldControlProps = {
  id?: string;
  'aria-describedby'?: string;
};

export interface AgentOSFieldProps {
  id: string;
  label: string;
  children: ReactNode;
  hint?: string;
  error?: string;
  required?: boolean;
  className?: string;
}

export function AgentOSField({
  id,
  label,
  children,
  hint,
  error,
  required = false,
  className,
}: AgentOSFieldProps) {
  const descriptionId = `${id}-description`;
  const hasDescription = Boolean(hint || error);
  const control = React.isValidElement<FieldControlProps>(children)
    ? React.cloneElement(children, {
        id: children.props.id ?? id,
        'aria-describedby':
          [
            children.props['aria-describedby'],
            hasDescription ? descriptionId : undefined,
          ]
            .filter(Boolean)
            .join(' ') || undefined,
      })
    : children;

  return (
    <div className={cn('agentos-field', className)}>
      <label className="agentos-field__label" htmlFor={id}>
        {label}
        {required && (
          <span className="agentos-field__required" aria-hidden="true">
            *
          </span>
        )}
      </label>
      {control}
      {hasDescription && (
        <p
          id={descriptionId}
          className={error ? 'agentos-field__error' : 'agentos-field__hint'}
          role={error ? 'alert' : undefined}
        >
          {error ?? hint}
        </p>
      )}
    </div>
  );
}
