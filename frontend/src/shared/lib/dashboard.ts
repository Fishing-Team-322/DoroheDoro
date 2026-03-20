import {
  alertStatusMeta,
  deploymentStatusMeta,
  healthMeta,
  hostStatusMeta,
  severityMeta,
} from "@/src/shared/constants/dashboard";
import type {
  Alert,
  AlertStatus,
  DeploymentJob,
  DeploymentStatus,
  HealthState,
  Host,
  HostStatus,
  Severity,
} from "@/src/shared/types/dashboard";

export function getHostStatusMeta(status: HostStatus) {
  return hostStatusMeta[status];
}

export function getDeploymentStatusMeta(status: DeploymentStatus) {
  return deploymentStatusMeta[status];
}

export function getAlertStatusMeta(status: AlertStatus) {
  return alertStatusMeta[status];
}

export function getSeverityMeta(severity: Severity) {
  return severityMeta[severity];
}

export function getHealthMeta(health: HealthState) {
  return healthMeta[health];
}

export function formatRelativeLabel(isoDate: string) {
  const date = new Date(isoDate);
  return new Intl.DateTimeFormat("ru-RU", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function formatPercent(value: number) {
  return `${Math.round(value)}%`;
}

export function formatCompactNumber(value: number) {
  return new Intl.NumberFormat("ru-RU", {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
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
