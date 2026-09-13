import type { ReactNode } from 'react';
import { cn } from '../lib/cn';
import { CaretDownIcon, CheckIcon, type Icon } from '@phosphor-icons/react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from './Dropdown';

export interface PropertyDropdownOption<T extends string = string> {
  value: T;
  label: string;
  renderOption?: () => ReactNode;
}

export interface PropertyDropdownProps<T extends string = string> {
  value: T;
  options: PropertyDropdownOption<T>[];
  onChange: (value: T) => void;
  icon?: Icon;
  label?: string;
  disabled?: boolean;
  /** Show only icon without label, value, or caret */
  iconOnly?: boolean;
  /** Value considered "default" (no highlight in icon-only mode). Defaults to first option. */
  defaultValue?: T;
}

export function PropertyDropdown<T extends string = string>({
  value,
  options,
  onChange,
  icon: IconComponent,
  label,
  disabled,
  iconOnly,
  defaultValue,
}: PropertyDropdownProps<T>) {
  const selectedOption = options.find((opt) => opt.value === value);
  const isNonDefault = value !== (defaultValue ?? options[0]?.value);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild disabled={disabled}>
        <button
          type="button"
          className={cn(
            'flex items-center gap-half bg-panel rounded-sm',
            'text-sm text-normal hover:bg-secondary transition-colors',
            'disabled:opacity-50 disabled:cursor-not-allowed',
            'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand',
            'py-half',
            'px-base',
            iconOnly && isNonDefault && 'text-brand'
          )}
          aria-label={iconOnly ? (label ?? selectedOption?.label) : undefined}
        >
          {iconOnly && IconComponent ? (
            <IconComponent
              className="size-icon-xs"
              weight="bold"
              aria-hidden="true"
            />
          ) : IconComponent ? (
            <>
              <IconComponent
                className="size-icon-xs"
                weight="bold"
                aria-hidden="true"
              />
              {label && <span>{label}:</span>}
              <span>{selectedOption?.label}</span>
            </>
          ) : (
            (selectedOption?.renderOption?.() ?? selectedOption?.label)
          )}
          {!iconOnly && (
            <CaretDownIcon
              className="size-icon-2xs text-low"
              weight="bold"
              aria-hidden="true"
            />
          )}
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        {options.map((option) => (
          <DropdownMenuItem
            key={option.value}
            onClick={() => onChange(option.value)}
            badge={
              option.value === value ? (
                <CheckIcon className="size-icon-xs text-brand" weight="bold" />
              ) : undefined
            }
          >
            {option.renderOption?.() ?? option.label}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
