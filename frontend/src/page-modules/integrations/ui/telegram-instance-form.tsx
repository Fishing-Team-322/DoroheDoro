"use client";

import { FormEvent } from "react";
import { translateValueLabel, useI18n } from "@/src/shared/lib/i18n";
import { Button, Input, Select, Switch } from "@/src/shared/ui";
import {
  TextAreaField,
} from "@/src/features/operations/ui/operations-ui";
import type { TelegramInstanceDraft } from "@/src/shared/lib/telegram-integrations-store";
import { ClusterBindingEditor } from "./cluster-binding-editor";

const copyByLocale = {
  en: {
    instanceName: "Instance name",
    instanceNamePlaceholder: "Primary Ops Bot",
    defaultChatId: "Default chat id",
    defaultChatIdPlaceholder: "-10025001001",
    botToken: "Bot token",
    botTokenPlaceholder: "750001:AA-demo-ops-primary",
    statusAria: "Telegram instance status",
    enabled: "Instance enabled",
    paused: "Instance paused",
    notes: "Operator notes",
    notesPlaceholder: "Routing notes, ownership, escalation hints...",
    bindingsTitle: "Cluster bindings",
    bindingsDescription:
      "Bind one instance to multiple clusters or operator routes without touching backend contracts.",
    testConnection: "Test connection",
    deleteInstance: "Delete instance",
  },
  ru: {
    instanceName: "Имя инстанса",
    instanceNamePlaceholder: "Primary Ops Bot",
    defaultChatId: "Chat ID по умолчанию",
    defaultChatIdPlaceholder: "-10025001001",
    botToken: "Токен бота",
    botTokenPlaceholder: "750001:AA-demo-ops-primary",
    statusAria: "Статус Telegram-инстанса",
    enabled: "Инстанс включен",
    paused: "Инстанс на паузе",
    notes: "Заметки оператора",
    notesPlaceholder: "Заметки по маршрутизации, ownership, подсказки по эскалации...",
    bindingsTitle: "Привязки кластеров",
    bindingsDescription:
      "Привяжите один инстанс к нескольким кластерам или операторским маршрутам, не меняя backend-контракты.",
    testConnection: "Проверить подключение",
    deleteInstance: "Удалить инстанс",
  },
} as const;

export function TelegramInstanceForm({
  draft,
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
  draft: TelegramInstanceDraft;
  onChange: (value: TelegramInstanceDraft) => void;
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
          label={copy.defaultChatId}
          value={draft.defaultChatId}
          onChange={(event) =>
            onChange({ ...draft, defaultChatId: event.target.value })
          }
          placeholder={copy.defaultChatIdPlaceholder}
        />
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Input
          label={copy.botToken}
          value={draft.botToken}
          onChange={(event) => onChange({ ...draft, botToken: event.target.value })}
          placeholder={copy.botTokenPlaceholder}
        />

        <div className="space-y-2">
          <Select
            value={draft.status}
            onChange={(event) =>
              onChange({
                ...draft,
                status: event.target.value as TelegramInstanceDraft["status"],
              })
            }
            options={[
              { value: "active", label: translateValueLabel("active", locale) },
              { value: "paused", label: translateValueLabel("paused", locale) },
              { value: "degraded", label: translateValueLabel("degraded", locale) },
            ]}
            selectSize="lg"
            aria-label={copy.statusAria}
          />
          <Switch
            checked={draft.enabled}
            onCheckedChange={(checked) => onChange({ ...draft, enabled: checked })}
            switchLabel={draft.enabled ? copy.enabled : copy.paused}
          />
        </div>
      </div>

      <TextAreaField
        id="telegram-notes"
        label={copy.notes}
        value={draft.notes}
        onChange={(event) => onChange({ ...draft, notes: event.target.value })}
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
        />
      </div>

      {formError ? (
        <p className="text-sm text-[color:var(--status-danger-fg)]">{formError}</p>
      ) : null}

      <div className="flex flex-wrap gap-3">
        <Button type="button" variant="outline" loading={testing} onClick={onTestConnection}>
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
