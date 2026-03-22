"use client";

import type {
  AnomalyMode,
  AnomalyModeDefinition,
} from "@/src/shared/lib/operations-workbench";

type Tone = {
  iconClassName: string;
  hoverClassName: string;
  activeClassName: string;
};

function cn(...classes: Array<string | false | null | undefined>) {
  return classes.filter(Boolean).join(" ");
}

function FlameIcon({ className }: { className?: string }) {
  const pathD =
    "M12 3q1 4 4 6.5t3 5.5a1 1 0 0 1-14 0 5 5 0 0 1 1-3 1 1 0 0 0 5 0c0-2-1.5-3-1.5-5q0-2 2.5-4";

  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      className={className}
      aria-hidden="true"
    >
      <path d={pathD} fill="currentColor" />
      <path
        d={pathD}
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function getTone(mode: string): Tone {
  switch (mode.toLowerCase()) {
    case "light":
      return {
        iconClassName: "text-sky-400",
        hoverClassName: "hover:border-sky-500/30 hover:bg-sky-500/[0.03]",
        activeClassName:
          "border-sky-500/40 bg-sky-500/5 shadow-[inset_0_0_0_1px_rgba(14,165,233,0.14)]",
      };

    case "medium":
      return {
        iconClassName: "text-amber-400",
        hoverClassName: "hover:border-amber-500/30 hover:bg-amber-500/[0.03]",
        activeClassName:
          "border-amber-500/40 bg-amber-500/5 shadow-[inset_0_0_0_1px_rgba(245,158,11,0.14)]",
      };

    case "heavy":
      return {
        iconClassName: "text-rose-400",
        hoverClassName: "hover:border-rose-500/30 hover:bg-rose-500/[0.03]",
        activeClassName:
          "border-rose-500/40 bg-rose-500/5 shadow-[inset_0_0_0_1px_rgba(244,63,94,0.14)]",
      };

    default:
      return {
        iconClassName: "text-[color:var(--muted-foreground)]",
        hoverClassName:
          "hover:border-[color:var(--border-strong,rgba(255,255,255,0.16))] hover:bg-[color:var(--surface-subtle)]",
        activeClassName:
          "border-[color:var(--status-info-border)] bg-[color:var(--status-info-bg)]/30 shadow-[inset_0_0_0_1px_rgba(56,189,248,0.12)]",
      };
  }
}

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
        const tone = getTone(String(option.id));

        return (
          <button
            key={option.id}
            type="button"
            onClick={() => onChange(option.id)}
            aria-pressed={active}
            className={cn(
              "flex flex-col justify-start rounded-2xl border p-5 text-left transition-all duration-200",
              "bg-[color:var(--surface)]",
              "border-[color:var(--border)]",
              "focus:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--status-info-border)]/40",
              !active && tone.hoverClassName,
              active && tone.activeClassName
            )}
          >
            <div className="min-h-[36px]">
              <div className="inline-flex items-start gap-2 align-top">
                <h3 className="text-[28px] font-semibold leading-[1] tracking-tight text-[color:var(--foreground)]">
                  {option.label}
                </h3>
                <FlameIcon
                  className={cn(
                    "mt-[3px] h-[18px] w-[18px] shrink-0",
                    tone.iconClassName
                  )}
                />
              </div>
            </div>

            <p className="mt-3 text-lg font-medium leading-6 text-[color:var(--foreground)]">
              {option.subtitle}
            </p>

            <p className="mt-4 text-base leading-7 text-[color:var(--muted-foreground)]">
              {option.description}
            </p>
          </button>
        );
      })}
    </div>
  );
}
