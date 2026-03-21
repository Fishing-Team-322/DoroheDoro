import type {
  BadgeTone,
  HealthState,
  HostStatus,
} from "@/src/shared/types/dashboard";

export const badgeToneClassNames: Record<BadgeTone, string> = {
  neutral:
    "border-[color:var(--status-neutral-border)] bg-[color:var(--status-neutral-bg)] text-[color:var(--status-neutral-fg)]",
  info:
    "border-[color:var(--status-info-border)] bg-[color:var(--status-info-bg)] text-[color:var(--status-info-fg)]",
  positive:
    "border-[color:var(--status-positive-border)] bg-[color:var(--status-positive-bg)] text-[color:var(--status-positive-fg)]",
  warning:
    "border-[color:var(--status-warning-border)] bg-[color:var(--status-warning-bg)] text-[color:var(--status-warning-fg)]",
  danger:
    "border-[color:var(--status-danger-border)] bg-[color:var(--status-danger-bg)] text-[color:var(--status-danger-fg)]",
};

export const hostStatusTones: Record<HostStatus, BadgeTone> = {
  online: "positive",
  offline: "danger",
  degraded: "warning",
  enrolling: "info",
};

export const healthTones: Record<HealthState, BadgeTone> = {
  healthy: "positive",
  warning: "warning",
  critical: "danger",
  unknown: "neutral",
};

export const environmentValues = ["all", "prod", "staging", "dev"] as const;
