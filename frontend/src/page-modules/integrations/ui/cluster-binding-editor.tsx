"use client";

import { Button, Card, Input, Select, Switch } from "@/src/shared/ui";
import {
  createEmptyBinding,
  type TelegramClusterBinding,
  type TelegramSeverityLevel,
} from "@/src/shared/lib/telegram-integrations-store";

const SEVERITY_PRESETS: Array<{
  value: string;
  label: string;
  severities: TelegramSeverityLevel[];
}> = [
  { value: "critical-only", label: "Critical only", severities: ["critical"] },
  {
    value: "warning-plus",
    label: "Warning + Critical",
    severities: ["warning", "critical"],
  },
  {
    value: "full",
    label: "Info + Warning + Critical",
    severities: ["info", "warning", "critical"],
  },
];

function getPresetValue(severities: TelegramSeverityLevel[]) {
  return (
    SEVERITY_PRESETS.find(
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
                Binding {index + 1}
              </p>
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Route one Telegram instance to a cluster/operator scope.
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
                switchLabel={binding.enabled ? "Enabled" : "Paused"}
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
                Remove
              </Button>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <Input
              label="Cluster"
              value={binding.cluster}
              onChange={(event) =>
                updateBinding(binding.id, (current) => ({
                  ...current,
                  cluster: event.target.value,
                }))
              }
              placeholder="core / edge / security"
            />
            <Input
              label="Route label"
              value={binding.routeLabel}
              onChange={(event) =>
                updateBinding(binding.id, (current) => ({
                  ...current,
                  routeLabel: event.target.value,
                }))
              }
              placeholder="core-oncall"
            />
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            <Input
              label="Chat ID"
              value={binding.chatId}
              onChange={(event) =>
                updateBinding(binding.id, (current) => ({
                  ...current,
                  chatId: event.target.value,
                }))
              }
              placeholder="-10025001001"
            />

            <Select
              value={getPresetValue(binding.severities)}
              onChange={(event) => {
                const preset =
                  SEVERITY_PRESETS.find((item) => item.value === event.target.value) ??
                  SEVERITY_PRESETS[1];
                updateBinding(binding.id, (current) => ({
                  ...current,
                  severities: preset.severities,
                }));
              }}
              options={SEVERITY_PRESETS.map((item) => ({
                value: item.value,
                label: item.label,
              }))}
              selectSize="lg"
              aria-label={`Binding ${index + 1} severity profile`}
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
        Add binding
      </Button>
    </div>
  );
}
