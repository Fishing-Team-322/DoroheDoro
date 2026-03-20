import type {
  AlertStatus,
  BadgeTone,
  DeploymentStatus,
  HealthState,
  HostStatus,
  Severity,
  StatusMeta,
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

export const hostStatusMeta: StatusMeta<HostStatus> = {
  online: { label: "В сети", tone: "positive" },
  offline: { label: "Недоступен", tone: "danger" },
  degraded: { label: "Снижен", tone: "warning" },
  enrolling: { label: "Подключается", tone: "info" },
};

export const deploymentStatusMeta: StatusMeta<DeploymentStatus> = {
  pending: { label: "Ожидает", tone: "neutral" },
  running: { label: "Выполняется", tone: "info" },
  success: { label: "Успешно", tone: "positive" },
  failed: { label: "Ошибка", tone: "danger" },
  canceled: { label: "Отменено", tone: "warning" },
};

export const alertStatusMeta: StatusMeta<AlertStatus> = {
  active: { label: "Активно", tone: "danger" },
  acknowledged: { label: "Подтверждено", tone: "warning" },
  resolved: { label: "Решено", tone: "positive" },
  muted: { label: "Приглушено", tone: "neutral" },
};

export const severityMeta: StatusMeta<Severity> = {
  debug: { label: "Отладка", tone: "neutral" },
  info: { label: "Инфо", tone: "info" },
  warning: { label: "Предупреждение", tone: "warning" },
  error: { label: "Ошибка", tone: "danger" },
  critical: { label: "Критично", tone: "danger" },
};

export const healthMeta: StatusMeta<HealthState> = {
  healthy: { label: "Нормально", tone: "positive" },
  warning: { label: "Предупреждение", tone: "warning" },
  critical: { label: "Критично", tone: "danger" },
  unknown: { label: "Неизвестно", tone: "neutral" },
};

export const environmentOptions = [
  { label: "Все окружения", value: "all" },
  { label: "Прод", value: "prod" },
  { label: "Стейдж", value: "staging" },
  { label: "Разработка", value: "dev" },
] as const;

export const timeRangeOptions = [
  { label: "Последние 15 минут", value: "15m" },
  { label: "Последний час", value: "1h" },
  { label: "Последние 6 часов", value: "6h" },
  { label: "Последние 24 часа", value: "24h" },
  { label: "Последние 7 дней", value: "7d" },
] as const;
