import {
  healthTones,
  hostStatusTones,
} from "@/src/shared/constants/dashboard";
import type {
  Alert,
  DeploymentJob,
  HealthState,
  Host,
  HostStatus,
} from "@/src/shared/types/dashboard";
import type { Locale } from "@/src/shared/config";

export function getHostStatusMeta(
  status: HostStatus,
  labels: Record<HostStatus, string>
) {
  return {
    label: labels[status],
    tone: hostStatusTones[status],
  };
}

export function getHealthMeta(
  health: HealthState,
  labels: Record<HealthState, string>
) {
  return {
    label: labels[health],
    tone: healthTones[health],
  };
}

export function formatRelativeLabel(isoDate: string, locale: Locale) {
  const date = new Date(isoDate);
  const intlLocale = locale === "en" ? "en-US" : "ru-RU";

  return new Intl.DateTimeFormat(intlLocale, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function formatPercent(value: number) {
  return `${Math.round(value)}%`;
}

export function countHostsByStatus(hosts: Host[]) {
  return hosts.reduce(
    (acc, host) => {
      acc[host.status] += 1;
      return acc;
    },
    {
      online: 0,
      offline: 0,
      degraded: 0,
      enrolling: 0,
    }
  );
}

export function countAlertsByStatus(alerts: Alert[]) {
  return alerts.reduce(
    (acc, alert) => {
      acc[alert.status] += 1;
      return acc;
    },
    {
      active: 0,
      acknowledged: 0,
      resolved: 0,
      muted: 0,
    }
  );
}

export function countJobsByStatus(jobs: DeploymentJob[]) {
  return jobs.reduce(
    (acc, job) => {
      acc[job.status] += 1;
      return acc;
    },
    {
      pending: 0,
      running: 0,
      success: 0,
      failed: 0,
      canceled: 0,
    }
  );
}
