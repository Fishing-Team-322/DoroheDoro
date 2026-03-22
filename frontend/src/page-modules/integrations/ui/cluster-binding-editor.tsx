"use client";

import { useI18n } from "@/src/shared/lib/i18n";
import {
  createEmptyTelegramBindingDraft,
  TELEGRAM_EVENT_TYPES,
  TELEGRAM_SEVERITY_THRESHOLDS,
  toggleTelegramEventType,
  type TelegramBindingDraft,
  type TelegramBindingEventType,
} from "@/src/shared/lib/telegram-integrations";
import { Button, Card, Select, Switch } from "@/src/shared/ui";
import type { ClusterItem } from "@/src/shared/lib/runtime-api";

const copyByLocale = {
  en: {
    bindingPrefix: "Binding",
    bindingDescription:
      "Bind Telegram delivery to a global scope or one cluster.",
    enabled: "Enabled",
    paused: "Paused",
    remove: "Remove",
    scope: "Scope",
    global: "Global",
    cluster: "Cluster",
    clusterLabel: "Cluster scope",
    clusterPlaceholder: "Pick a cluster",
    severity: "Minimum severity",
    events: "Event types",
    add: "Add binding",
  },
  ru: {
    bindingPrefix: "Привязка",
    bindingDescription:
      "Привяжите доставку Telegram к global scope или к одному кластеру.",
    enabled: "Включено",
    paused: "Пауза",
    remove: "Удалить",
    scope: "Скоуп",
    global: "Global",
    cluster: "Кластер",
    clusterLabel: "Кластерный scope",
    clusterPlaceholder: "Выберите кластер",
    severity: "Минимальная severity",
    events: "Типы событий",
    add: "Добавить привязку",
  },
} as const;

function buildClusterOptions(
  locale: "ru" | "en",
  clusters: ClusterItem[]
): Array<{ value: string; label: string }> {
  const copy = copyByLocale[locale];
  return [
    { value: "", label: copy.clusterPlaceholder },
    ...clusters.map((cluster) => ({
      value: cluster.cluster_id,
      label: `${cluster.name} (${cluster.slug})`,
    })),
  ];
}

export function ClusterBindingEditor({
  value,
  onChange,
  clusters,
}: {
  value: TelegramBindingDraft[];
  onChange: (value: TelegramBindingDraft[]) => void;
  clusters: ClusterItem[];
}) {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
  const bindings =
    value.length > 0 ? value : [createEmptyTelegramBindingDraft()];
  const clusterOptions = buildClusterOptions(locale, clusters);

  const updateBinding = (
    bindingIndex: number,
    updater: (binding: TelegramBindingDraft) => TelegramBindingDraft
  ) => {
    onChange(
      bindings.map((binding, index) =>
        index === bindingIndex ? updater(binding) : binding
      )
    );
  };

  return (
    <div className="space-y-3">
      {bindings.map((binding, index) => (
        <Card key={binding.id ?? `binding-${index}`} className="space-y-4 p-4">
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
                checked={binding.isActive}
                onCheckedChange={(checked) =>
                  updateBinding(index, (current) => ({
                    ...current,
                    isActive: checked,
                  }))
                }
                switchLabel={binding.isActive ? copy.enabled : copy.paused}
              />
              <Button
                variant="ghost"
                size="sm"
                className="h-10 px-3"
                onClick={() =>
                  onChange(
                    bindings.length === 1
                      ? [createEmptyTelegramBindingDraft()]
                      : bindings.filter(
                          (_, currentIndex) => currentIndex !== index
                        )
                  )
                }
              >
                {copy.remove}
              </Button>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-3">
            <div className="space-y-2">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.scope}
              </p>
              <div className="inline-flex gap-2">
                <Button
                  type="button"
                  variant={
                    binding.scopeType === "global" ? "default" : "outline"
                  }
                  size="sm"
                  onClick={() =>
                    updateBinding(index, (current) => ({
                      ...current,
                      scopeType: "global",
                      scopeId: "",
                    }))
                  }
                >
                  {copy.global}
                </Button>
                <Button
                  type="button"
                  variant={
                    binding.scopeType === "cluster" ? "default" : "outline"
                  }
                  size="sm"
                  onClick={() =>
                    updateBinding(index, (current) => ({
                      ...current,
                      scopeType: "cluster",
                    }))
                  }
                >
                  {copy.cluster}
                </Button>
              </div>
            </div>

            <div className="space-y-2">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.clusterLabel}
              </p>
              <Select
                value={binding.scopeType === "cluster" ? binding.scopeId : ""}
                onChange={(event) =>
                  updateBinding(index, (current) => ({
                    ...current,
                    scopeId: event.target.value,
                    scopeType: event.target.value
                      ? "cluster"
                      : current.scopeType,
                  }))
                }
                options={clusterOptions}
                selectSize="lg"
                disabled={binding.scopeType !== "cluster"}
              />
            </div>

            <div className="space-y-2">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.severity}
              </p>
              <Select
                value={binding.severityThreshold}
                onChange={(event) =>
                  updateBinding(index, (current) => ({
                    ...current,
                    severityThreshold: event.target
                      .value as TelegramBindingDraft["severityThreshold"],
                  }))
                }
                options={TELEGRAM_SEVERITY_THRESHOLDS.map((severity) => ({
                  value: severity,
                  label: severity,
                }))}
                selectSize="lg"
              />
            </div>
          </div>

          <div className="space-y-2">
            <p className="text-sm text-[color:var(--muted-foreground)]">
              {copy.events}
            </p>
            <div className="flex flex-wrap gap-2">
              {TELEGRAM_EVENT_TYPES.map((eventType) => {
                const selected = binding.eventTypes.includes(eventType);

                return (
                  <Button
                    key={eventType}
                    type="button"
                    variant={selected ? "default" : "outline"}
                    size="sm"
                    onClick={() =>
                      updateBinding(index, (current) => ({
                        ...current,
                        eventTypes: toggleTelegramEventType(
                          current.eventTypes,
                          eventType as TelegramBindingEventType
                        ),
                      }))
                    }
                  >
                    {eventType}
                  </Button>
                );
              })}
            </div>
          </div>
        </Card>
      ))}

      <Button
        variant="outline"
        size="sm"
        className="h-10 px-4"
        onClick={() =>
          onChange([...bindings, createEmptyTelegramBindingDraft()])
        }
      >
        {copy.add}
      </Button>
    </div>
  );
}
