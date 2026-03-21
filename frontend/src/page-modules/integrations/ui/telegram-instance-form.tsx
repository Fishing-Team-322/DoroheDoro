"use client";

import { FormEvent } from "react";
import { Button, Input, Select, Switch } from "@/src/shared/ui";
import {
  TextAreaField,
} from "@/src/features/operations/ui/operations-ui";
import type { TelegramInstanceDraft } from "@/src/shared/lib/telegram-integrations-store";
import { ClusterBindingEditor } from "./cluster-binding-editor";

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
  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    onSubmit();
  };

  return (
    <form className="space-y-5" onSubmit={handleSubmit}>
      <div className="grid gap-4 md:grid-cols-2">
        <Input
          label="Instance name"
          value={draft.name}
          onChange={(event) => onChange({ ...draft, name: event.target.value })}
          placeholder="Primary Ops Bot"
        />
        <Input
          label="Default chat id"
          value={draft.defaultChatId}
          onChange={(event) =>
            onChange({ ...draft, defaultChatId: event.target.value })
          }
          placeholder="-10025001001"
        />
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <Input
          label="Bot token"
          value={draft.botToken}
          onChange={(event) => onChange({ ...draft, botToken: event.target.value })}
          placeholder="750001:AA-demo-ops-primary"
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
              { value: "active", label: "Active" },
              { value: "paused", label: "Paused" },
              { value: "degraded", label: "Degraded" },
            ]}
            selectSize="lg"
            aria-label="Telegram instance status"
          />
          <Switch
            checked={draft.enabled}
            onCheckedChange={(checked) => onChange({ ...draft, enabled: checked })}
            switchLabel={draft.enabled ? "Instance enabled" : "Instance paused"}
          />
        </div>
      </div>

      <TextAreaField
        id="telegram-notes"
        label="Operator notes"
        value={draft.notes}
        onChange={(event) => onChange({ ...draft, notes: event.target.value })}
        placeholder="Routing notes, ownership, escalation hints..."
      />

      <div className="space-y-3">
        <div>
          <p className="text-sm font-semibold text-[color:var(--foreground)]">
            Cluster bindings
          </p>
          <p className="text-sm text-[color:var(--muted-foreground)]">
            Bind one instance to multiple clusters or operator routes without touching backend contracts.
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
          Test connection
        </Button>
        <Button type="submit" loading={saving}>
          {saveLabel}
        </Button>
        {deletable && onDelete ? (
          <Button type="button" variant="ghost" onClick={onDelete}>
            Delete instance
          </Button>
        ) : null}
      </div>
    </form>
  );
}
