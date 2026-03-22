"use client";

import { FormEvent } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
import { Button, Input, Select, Switch } from "@/src/shared/ui";
import { TextAreaField } from "@/src/features/operations/ui/operations-ui";
import {
  TELEGRAM_PARSE_MODES,
  type TelegramIntegrationDraft,
} from "@/src/shared/lib/telegram-integrations";
import type { ClusterItem } from "@/src/shared/lib/runtime-api";
import { ClusterBindingEditor } from "./cluster-binding-editor";

const copyByLocale = {
  en: {
    instanceName: "Integration name",
    instanceNamePlaceholder: "ops-telegram",
    botName: "Bot display name",
    botNamePlaceholder: "Primary Ops Bot",
    defaultChatId: "Default chat id",
    defaultChatIdPlaceholder: "-10025001001",
    secretRef: "Vault secret ref",
    secretRefPlaceholder: "vault://secret/data/integrations/tg/ops-primary",
    secretRefHint:
      "Required when creating a new Telegram integration or rotating secret material.",
    parseMode: "Parse mode",
    integrationEnabled: "Integration active",
    integrationPaused: "Integration paused",
    deliveryEnabled: "Telegram delivery enabled",
    deliveryDisabled: "Telegram delivery paused",
    notes: "Operator notes",
    notesPlaceholder: "Ownership, escalation notes, change reason...",
    bindingsTitle: "Delivery bindings",
    bindingsDescription:
      "Bindings now follow the live backend contract: scope, event types, and severity threshold.",
    testConnection: "Send healthcheck",
    deleteInstance: "Delete instance",
    maskedSecretRef: "Current secret ref",
  },
  ru: {
    instanceName: "Имя интеграции",
    instanceNamePlaceholder: "ops-telegram",
    botName: "Отображаемое имя бота",
    botNamePlaceholder: "Primary Ops Bot",
    defaultChatId: "Chat ID по умолчанию",
    defaultChatIdPlaceholder: "-10025001001",
    secretRef: "Vault secret ref",
    secretRefPlaceholder: "vault://secret/data/integrations/tg/ops-primary",
    secretRefHint:
      "Нужен при создании новой Telegram-интеграции или при ротации секрета.",
    parseMode: "Parse mode",
    integrationEnabled: "Интеграция активна",
    integrationPaused: "Интеграция на паузе",
    deliveryEnabled: "Доставка в Telegram включена",
    deliveryDisabled: "Доставка в Telegram приостановлена",
    notes: "Заметки оператора",
    notesPlaceholder: "Ownership, эскалация, причина изменения...",
    bindingsTitle: "Привязки доставки",
    bindingsDescription:
      "Привязки теперь совпадают с живым backend-контрактом: scope, event types и severity threshold.",
    testConnection: "Отправить healthcheck",
    deleteInstance: "Удалить инстанс",
    maskedSecretRef: "Текущий secret ref",
  },
} as const;

export function TelegramInstanceForm({
  draft,
  clusters,
  onChange,
  onSubmit,
  onTestConnection,
  onDelete,
  formError,
  saveLabel,
  testing,
  saving,
  deletable,
}: {
  draft: TelegramIntegrationDraft;
  clusters: ClusterItem[];
  onChange: (value: TelegramIntegrationDraft) => void;
  onSubmit: () => void;
  onTestConnection: () => void;
  onDelete?: () => void;
  formError?: string;
  saveLabel: string;
  testing: boolean;
  saving: boolean;
  deletable: boolean;
}) {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    onSubmit();
  };

  return (
    <form className="space-y-5" onSubmit={handleSubmit}>
      <div className="grid gap-4 md:grid-cols-2">
        <Input
          label={copy.instanceName}
          value={draft.name}
          onChange={(event) => onChange({ ...draft, name: event.target.value })}
          placeholder={copy.instanceNamePlaceholder}
        />
        <Input
          label={copy.botName}
          value={draft.botName}
          onChange={(event) =>
            onChange({ ...draft, botName: event.target.value })
          }
          placeholder={copy.botNamePlaceholder}
        />
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Input
          label={copy.defaultChatId}
          value={draft.defaultChatId}
          onChange={(event) =>
            onChange({ ...draft, defaultChatId: event.target.value })
          }
          placeholder={copy.defaultChatIdPlaceholder}
        />
        <div className="space-y-2">
          <Select
            value={draft.parseMode}
            onChange={(event) =>
              onChange({
                ...draft,
                parseMode: event.target
                  .value as TelegramIntegrationDraft["parseMode"],
              })
            }
            options={TELEGRAM_PARSE_MODES.map((mode) => ({
              value: mode,
              label: mode,
            }))}
            selectSize="lg"
            aria-label={copy.parseMode}
          />
        </div>
      </div>

      <div className="space-y-2">
        <Input
          label={copy.secretRef}
          value={draft.secretRef}
          onChange={(event) =>
            onChange({ ...draft, secretRef: event.target.value })
          }
          placeholder={copy.secretRefPlaceholder}
        />
        <p className="text-sm text-[color:var(--muted-foreground)]">
          {copy.secretRefHint}
        </p>
        {draft.maskedSecretRef ? (
          <p className="text-sm text-[color:var(--muted-foreground)]">
            {copy.maskedSecretRef}: {draft.maskedSecretRef}
          </p>
        ) : null}
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Switch
          checked={draft.isActive}
          onCheckedChange={(checked) =>
            onChange({ ...draft, isActive: checked })
          }
          switchLabel={
            draft.isActive ? copy.integrationEnabled : copy.integrationPaused
          }
        />
        <Switch
          checked={draft.deliveryEnabled}
          onCheckedChange={(checked) =>
            onChange({ ...draft, deliveryEnabled: checked })
          }
          switchLabel={
            draft.deliveryEnabled ? copy.deliveryEnabled : copy.deliveryDisabled
          }
        />
      </div>

      <TextAreaField
        id="telegram-notes"
        label={copy.notes}
        value={draft.description}
        onChange={(event) =>
          onChange({ ...draft, description: event.target.value })
        }
        placeholder={copy.notesPlaceholder}
      />

      <div className="space-y-3">
        <div>
          <p className="text-sm font-semibold text-[color:var(--foreground)]">
            {copy.bindingsTitle}
          </p>
          <p className="text-sm text-[color:var(--muted-foreground)]">
            {copy.bindingsDescription}
          </p>
        </div>

        <ClusterBindingEditor
          value={draft.bindings}
          onChange={(bindings) => onChange({ ...draft, bindings })}
          clusters={clusters}
        />
      </div>

      {formError ? (
        <p className="text-sm text-[color:var(--status-danger-fg)]">
          {formError}
        </p>
      ) : null}

      <div className="flex flex-wrap gap-3">
        <Button
          type="button"
          variant="outline"
          loading={testing}
          onClick={onTestConnection}
        >
          {copy.testConnection}
        </Button>
        <Button type="submit" loading={saving}>
          {saveLabel}
        </Button>
        {deletable && onDelete ? (
          <Button type="button" variant="ghost" onClick={onDelete}>
            {copy.deleteInstance}
          </Button>
        ) : null}
      </div>
    </form>
  );
}
