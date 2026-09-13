import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { CheckIcon, type Icon } from '@phosphor-icons/react';
import { cn } from '@/shared/lib/utils';
import { Tooltip } from '@vibe/ui/components/Tooltip';

interface CopyButtonProps {
  onCopy: () => void;
  disabled: boolean;
  iconSize: string;
  /** Icon to show before copying */
  icon: Icon;
}

/**
 * Copy button with self-contained feedback state.
 * Shows a checkmark for 2 seconds after copying.
 */
export function CopyButton({
  onCopy,
  disabled,
  iconSize,
  icon: DefaultIcon,
}: CopyButtonProps) {
  const { t } = useTranslation('common');
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(timer);
  }, [copied]);

  const handleClick = () => {
    onCopy();
    setCopied(true);
  };

  const IconComponent = copied ? CheckIcon : DefaultIcon;
  const tooltip = copied ? t('actions.copied') : t('actions.copyPath');
  const iconClassName = copied
    ? 'text-success hover:text-success group-hover:text-success'
    : undefined;

  const button = (
    <button
      type="button"
      className={cn(
        'flex items-center justify-center transition-colors',
        'drop-shadow-[2px_2px_4px_rgba(121,121,121,0.25)]',
        'min-h-7 min-w-7 rounded-sm text-low group-hover:text-normal',
        'hover:bg-secondary hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand'
      )}
      aria-label={tooltip}
      onClick={handleClick}
      disabled={disabled}
    >
      <IconComponent className={cn(iconSize, iconClassName)} weight="bold" />
    </button>
  );

  return (
    <Tooltip content={tooltip} side="top">
      {button}
    </Tooltip>
  );
}
