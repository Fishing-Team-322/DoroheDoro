"use client";

import type {
  AnomalyMode,
  AnomalyModeDefinition,
} from "@/src/shared/lib/operations-workbench";

export function DetectionModeSelector({
  value,
  options,
  onChange,
}: {
  value: AnomalyMode;
  options: AnomalyModeDefinition[];
  onChange: (value: AnomalyMode) => void;
}) {
  return (
    <div className="grid gap-3 md:grid-cols-3">
      {options.map((option) => {
        const active = option.id === value;

        return (
          <button
            key={option.id}
            type="button"
            onClick={() => onChange(option.id)}
            aria-pressed={active}
            className={`rounded-xl border p-4 text-left transition-colors ${
              active
                ? "border-[color:var(--status-info-border)] bg-[color:var(--status-info-bg)]/45"
                : "border-[color:var(--border)] bg-[color:var(--surface)] hover:bg-[color:var(--surface-subtle)]"
            }`}
          >
            <p className="text-base font-semibold text-[color:var(--foreground)]">
              {option.label}
            </p>
            <p className="mt-1 text-sm text-[color:var(--foreground)]">
              {option.subtitle}
            </p>
            <p className="mt-3 text-sm leading-6 text-[color:var(--muted-foreground)]">
              {option.description}
            </p>
          </button>
        );
      })}
    </div>
  );
}
