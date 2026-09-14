import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@vibe/ui/components/Button';
import { Input } from '@vibe/ui/components/Input';
import { Label } from '@vibe/ui/components/Label';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@vibe/ui/components/KeyboardDialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@vibe/ui/components/Select';
import { Alert, AlertDescription } from '@vibe/ui/components/Alert';
import { create, useModal } from '@ebay/nice-modal-react';
import { defineModal } from '@/shared/lib/modals';

export interface CreateConfigurationDialogProps {
  executorType: string;
  existingConfigs: string[];
}

export type CreateConfigurationResult = {
  action: 'created' | 'canceled';
  configName?: string;
  cloneFrom?: string | null;
};

const CreateConfigurationDialogImpl = create<CreateConfigurationDialogProps>(
  ({ executorType, existingConfigs }) => {
    const modal = useModal();
    const { t } = useTranslation(['settings', 'common']);
    const [configName, setConfigName] = useState('');
    const [cloneFrom, setCloneFrom] = useState<string | null>(null);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
      // Reset form when dialog opens
      if (modal.visible) {
        setConfigName('');
        setCloneFrom(null);
        setError(null);
      }
    }, [modal.visible]);

    const validateConfigName = (name: string): string | null => {
      const trimmedName = name.trim();
      if (!trimmedName) {
        return t('settings:settings.agents.createDialog.errors.nameRequired');
      }
      if (trimmedName.length > 40)
        return t('settings:settings.agents.createDialog.errors.tooLong');
      if (!/^[a-zA-Z0-9_-]+$/.test(trimmedName)) {
        return t(
          'settings:settings.agents.createDialog.errors.invalidCharacters'
        );
      }
      if (existingConfigs.includes(trimmedName)) {
        return t('settings:settings.agents.createDialog.errors.duplicate');
      }
      return null;
    };

    const handleCreate = () => {
      const validationError = validateConfigName(configName);
      if (validationError) {
        setError(validationError);
        return;
      }

      modal.resolve({
        action: 'created',
        configName: configName.trim(),
        cloneFrom,
      } as CreateConfigurationResult);
      modal.hide();
    };

    const handleCancel = () => {
      modal.resolve({ action: 'canceled' } as CreateConfigurationResult);
      modal.hide();
    };

    const handleOpenChange = (open: boolean) => {
      if (!open) {
        handleCancel();
      }
    };

    return (
      <Dialog open={modal.visible} onOpenChange={handleOpenChange}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {t('settings:settings.agents.createDialog.title')}
            </DialogTitle>
            <DialogDescription>
              {t('settings:settings.agents.createDialog.description', {
                executorType,
              })}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="config-name">
                {t('settings:settings.agents.createDialog.nameLabel')}
              </Label>
              <Input
                id="config-name"
                value={configName}
                onChange={(e) => {
                  setConfigName(e.target.value);
                  setError(null);
                }}
                placeholder={t(
                  'settings:settings.agents.createDialog.namePlaceholder'
                )}
                maxLength={40}
                autoFocus
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="clone-from">
                {t('settings:settings.agents.createDialog.cloneLabel')}
              </Label>
              <Select
                value={cloneFrom || '__blank__'}
                onValueChange={(value) =>
                  setCloneFrom(value === '__blank__' ? null : value)
                }
              >
                <SelectTrigger id="clone-from">
                  <SelectValue
                    placeholder={t(
                      'settings:settings.agents.createDialog.clonePlaceholder'
                    )}
                  />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__blank__">
                    {t('settings:settings.agents.createDialog.startBlank')}
                  </SelectItem>
                  {existingConfigs.map((configuration) => (
                    <SelectItem key={configuration} value={configuration}>
                      {t('settings:settings.agents.createDialog.cloneFrom', {
                        configuration,
                      })}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {error && (
              <Alert variant="destructive">
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={handleCancel}>
              {t('common:buttons.cancel')}
            </Button>
            <Button onClick={handleCreate} disabled={!configName.trim()}>
              {t('settings:settings.agents.createDialog.createButton')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const CreateConfigurationDialog = defineModal<
  CreateConfigurationDialogProps,
  CreateConfigurationResult
>(CreateConfigurationDialogImpl);
