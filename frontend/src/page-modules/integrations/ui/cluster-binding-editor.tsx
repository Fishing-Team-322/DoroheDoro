"use client";

import { useI18n } from "@/src/shared/lib/i18n";
import { Button, Card, Input, Select, Switch } from "@/src/shared/ui";
import {
  createEmptyBinding,
  type TelegramClusterBinding,
  type TelegramSeverityLevel,
} from "@/src/shared/lib/telegram-integrations-store";

const copyByLocale = {
  en: {
    presets: {
      criticalOnly: "Critical only",
      warningPlus: "Warning + Critical",
      full: "Info + Warning + Critical",
    },
    bindingPrefix: "Binding",
    bindingDescription:
      "Route one Telegram instance to a cluster/operator scope.",
    enabled: "Enabled",
    paused: "Paused",
    remove: "Remove",
    cluster: "Cluster",
    clusterPlaceholder: "core / edge / security",
    routeLabel: "Route label",
    routeLabelPlaceholder: "core-oncall",
    chatId: "Chat ID",
    chatIdPlaceholder: "-10025001001",
    severityProfileAria: "severity profile",
    add: "Add binding",
  },
  ru: {
    presets: {
      criticalOnly: "Только critical",
      warningPlus: "Warning + Critical",
      full: "Info + Warning + Critical",
    },
    bindingPrefix: "Привязка",
    bindingDescription:
      "Направьте один Telegram-инстанс в cluster/operator scope.",
    enabled: "Включено",
    paused: "Пауза",
    remove: "Удалить",
    cluster: "Кластер",
    clusterPlaceholder: "core / edge / security",
    routeLabel: "Метка маршрута",
    routeLabelPlaceholder: "core-oncall",
    chatId: "Chat ID",
    chatIdPlaceholder: "-10025001001",
    severityProfileAria: "профиль severity",
    add: "Добавить привязку",
  },
} as const;

function getPresetValue(
  severities: TelegramSeverityLevel[],
  presets: Array<{ value: string; severities: TelegramSeverityLevel[] }>
) {
  return (
    presets.find(
      (item) => item.severities.join("|") === severities.join("|")
    )?.value ?? "warning-plus"
  );
}

export function ClusterBindingEditor({
  value,
  onChange,
}: {
  value: TelegramClusterBinding[];
  onChange: (value: TelegramClusterBinding[]) => void;
}) {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
  const severityPresets: Array<{
    value: string;
    label: string;
    severities: TelegramSeverityLevel[];
  }> = [
    {
      value: "critical-only",
      label: copy.presets.criticalOnly,
      severities: ["critical"],
    },
    {
      value: "warning-plus",
      label: copy.presets.warningPlus,
      severities: ["warning", "critical"],
    },
    {
      value: "full",
      label: copy.presets.full,
      severities: ["info", "warning", "critical"],
    },
  ];
  const bindings = value.length > 0 ? value : [createEmptyBinding()];

  const updateBinding = (
    bindingId: string,
    updater: (binding: TelegramClusterBinding) => TelegramClusterBinding
  ) => {
    onChange(bindings.map((binding) => (binding.id === bindingId ? updater(binding) : binding)));
  };

  return (
    <div className="space-y-3">
      {bindings.map((binding, index) => (
        <Card key={binding.id} className="space-y-4 p-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p className="text-sm font-semibold text-[color:var(--foreground)]">
                {copy.bindingPrefix} {index + 1}
              </p>
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.bindingDescription}
              </p>
            </div>

            <div className="flex items-center gap-3">
              <Switch
                checked={binding.enabled}
                onCheckedChange={(checked) =>
                  updateBinding(binding.id, (current) => ({
                    ...current,
                    enabled: checked,
                  }))
                }
                switchLabel={binding.enabled ? copy.enabled : copy.paused}
              />
              <Button
                variant="ghost"
                size="sm"
                className="h-10 px-3"
                onClick={() =>
                  onChange(
                    bindings.length === 1
                      ? [createEmptyBinding()]
                      : bindings.filter((item) => item.id !== binding.id)
                  )
                }
              >
                {copy.remove}
              </Button>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <Input
              label={copy.cluster}
              value={binding.cluster}
              onChange={(event) =>
                updateBinding(binding.id, (current) => ({
                  ...current,
                  cluster: event.target.value,
                }))
              }
              placeholder={copy.clusterPlaceholder}
            />
            <Input
              label={copy.routeLabel}
              value={binding.routeLabel}
              onChange={(event) =>
                updateBinding(binding.id, (current) => ({
                  ...current,
                  routeLabel: event.target.value,
                }))
              }
              placeholder={copy.routeLabelPlaceholder}
            />
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <Input
              label={copy.chatId}
              value={binding.chatId}
              onChange={(event) =>
                updateBinding(binding.id, (current) => ({
                  ...current,
                  chatId: event.target.value,
                }))
              }
              placeholder={copy.chatIdPlaceholder}
            />

            <Select
              value={getPresetValue(binding.severities, severityPresets)}
              onChange={(event) => {
                const preset =
                  severityPresets.find((item) => item.value === event.target.value) ??
                  severityPresets[1];
                updateBinding(binding.id, (current) => ({
                  ...current,
                  severities: preset.severities,
                }));
              }}
              options={severityPresets.map((item) => ({
                value: item.value,
                label: item.label,
              }))}
              selectSize="lg"
              aria-label={`${copy.bindingPrefix} ${index + 1} ${copy.severityProfileAria}`}
            />
          </div>
        </Card>
      ))}

      <Button
        variant="outline"
        size="sm"
        className="h-10 px-4"
        onClick={() => onChange([...bindings, createEmptyBinding()])}
      >
        {copy.add}
      </Button>
    </div>
  );
}
