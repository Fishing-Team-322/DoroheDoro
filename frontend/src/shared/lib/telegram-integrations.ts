"use client";

import type {
  ClusterItem,
  IntegrationItem,
} from "@/src/shared/lib/runtime-api";

export const TELEGRAM_EVENT_TYPES = [
  "alerts.firing",
  "alerts.resolved",
  "anomalies.detected",
  "anomalies.resolved",
  "security.finding.opened",
  "security.finding.resolved",
] as const;

export const TELEGRAM_SEVERITY_THRESHOLDS = [
  "info",
  "low",
  "medium",
  "high",
  "critical",
] as const;

export const TELEGRAM_PARSE_MODES = ["HTML", "plain"] as const;

export type TelegramBindingEventType = (typeof TELEGRAM_EVENT_TYPES)[number];
export type TelegramSeverityThreshold =
  (typeof TELEGRAM_SEVERITY_THRESHOLDS)[number];
export type TelegramParseMode = (typeof TELEGRAM_PARSE_MODES)[number];
export type TelegramDeliveryStatus =
  | "queued"
  | "delivered"
  | "blocked"
  | "not-configured";

export type TelegramRuntimeBinding = {
  id: string;
  scopeType: "global" | "cluster";
  scopeId: string;
  cluster: string;
  routeLabel: string;
  chatId: string;
  eventTypes: TelegramBindingEventType[];
  severityThreshold: TelegramSeverityThreshold;
  enabled: boolean;
};

export type TelegramRuntimeInstanceStatus = "active" | "paused" | "degraded";

export type TelegramRuntimeInstance = {
  id: string;
  name: string;
  enabled: boolean;
  status: TelegramRuntimeInstanceStatus;
  notes: string;
  updatedAt: string;
  defaultChatId: string;
  maskedSecretRef?: string;
  hasSecretRef: boolean;
  bindings: TelegramRuntimeBinding[];
};

export type TelegramDeliveryProjection = {
  instanceId: string;
  instanceName: string;
  cluster: string;
  routeLabel: string;
  chatId: string;
  status: TelegramDeliveryStatus;
  detail: string;
};

export type TelegramBindingDraft = {
  id?: string;
  scopeType: "global" | "cluster";
  scopeId: string;
  eventTypes: TelegramBindingEventType[];
  severityThreshold: TelegramSeverityThreshold;
  isActive: boolean;
};

export type TelegramIntegrationDraft = {
  id?: string;
  name: string;
  description: string;
  botName: string;
  secretRef: string;
  maskedSecretRef?: string;
  hasSecretRef?: boolean;
  defaultChatId: string;
  parseMode: TelegramParseMode;
  deliveryEnabled: boolean;
  isActive: boolean;
  bindings: TelegramBindingDraft[];
};

export type TelegramRuntimeEvent = {
  id: string;
  event: string;
  integrationId?: string;
  requestId?: string;
  deliveryStatus?: string;
  classification?: string;
  messageId?: string;
  statusCode?: string;
  statusMessage?: string;
  createdAt?: string;
  raw: Record<string, unknown>;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value : undefined;
}

function asBoolean(value: unknown): boolean | undefined {
  return typeof value === "boolean" ? value : undefined;
}

function normalizeSeverityThreshold(
  value: string | undefined
): TelegramSeverityThreshold {
  switch (value?.trim().toLowerCase()) {
    case "critical":
      return "critical";
    case "high":
      return "high";
    case "warning":
    case "medium":
      return "medium";
    case "low":
      return "low";
    default:
      return "info";
  }
}

function normalizeEventType(
  value: string
): TelegramBindingEventType | undefined {
  return TELEGRAM_EVENT_TYPES.includes(value as TelegramBindingEventType)
    ? (value as TelegramBindingEventType)
    : undefined;
}

function severityRank(value: string | undefined): number {
  switch (normalizeSeverityThreshold(value)) {
    case "critical":
      return 4;
    case "high":
      return 3;
    case "medium":
      return 2;
    case "low":
      return 1;
    default:
      return 0;
  }
}

function normalizeAlertEventType(status?: string): TelegramBindingEventType {
  const normalized = status?.trim().toLowerCase() ?? "";
  return normalized === "resolved" || normalized === "closed"
    ? "alerts.resolved"
    : "alerts.firing";
}

function normalizeRuntimeStatus(input: {
  integrationActive: boolean;
  deliveryEnabled: boolean;
  hasSecretRef: boolean;
}): TelegramRuntimeInstanceStatus {
  if (!input.integrationActive || !input.deliveryEnabled) {
    return "paused";
  }
  if (!input.hasSecretRef) {
    return "degraded";
  }
  return "active";
}

export function buildTelegramRuntimeInstances(
  integrations: IntegrationItem[],
  clusters: ClusterItem[]
): TelegramRuntimeInstance[] {
  const clusterNames = new Map(
    clusters.map((cluster) => [cluster.cluster_id, cluster.name] as const)
  );

  return integrations
    .filter((item) => item.kind === "telegram_bot")
    .map((item) => {
      const config = isRecord(item.config_json) ? item.config_json : {};
      const defaultChatId = asString(config.default_chat_id) ?? "";
      const deliveryEnabled = asBoolean(config.delivery_enabled) ?? true;
      const hasSecretRef =
        asBoolean(config.has_secret_ref) ??
        Boolean(asString(config.masked_secret_ref));
      const bindings: TelegramRuntimeBinding[] = item.bindings.map(
        (binding): TelegramRuntimeBinding => {
          const scopeType: TelegramRuntimeBinding["scopeType"] =
            binding.scope_type === "cluster" ? "cluster" : "global";
          const clusterName =
            scopeType === "cluster"
              ? (clusterNames.get(binding.scope_id) ??
                binding.scope_id ??
                "cluster")
              : "global";
          const eventTypes: TelegramBindingEventType[] =
            binding.event_types_json
              .map((value) => normalizeEventType(value))
              .filter((value): value is TelegramBindingEventType =>
                Boolean(value)
              );

          return {
            id: binding.integration_binding_id,
            scopeType,
            scopeId: binding.scope_id ?? "",
            cluster: clusterName,
            routeLabel:
              scopeType === "cluster" ? "cluster-scope" : "global-scope",
            chatId: defaultChatId || "n/a",
            eventTypes: eventTypes.length > 0 ? eventTypes : ["alerts.firing"],
            severityThreshold: normalizeSeverityThreshold(
              binding.severity_threshold
            ),
            enabled: binding.is_active,
          };
        }
      );

      const runtimeInstance: TelegramRuntimeInstance = {
        id: item.integration_id,
        name: item.name,
        enabled: item.is_active && deliveryEnabled,
        status: normalizeRuntimeStatus({
          integrationActive: item.is_active,
          deliveryEnabled,
          hasSecretRef,
        }),
        notes: item.description ?? "",
        updatedAt: item.updated_at,
        defaultChatId,
        maskedSecretRef: asString(config.masked_secret_ref),
        hasSecretRef,
        bindings,
      };

      return runtimeInstance;
    })
    .sort((left, right) => left.name.localeCompare(right.name));
}

export function projectTelegramDeliveries(
  instances: TelegramRuntimeInstance[],
  input: {
    host?: string;
    service?: string;
    severity?: string;
    status?: string;
  }
): TelegramDeliveryProjection[] {
  const targetEventType = normalizeAlertEventType(input.status);
  const targetSeverityRank = severityRank(input.severity);

  const projections = instances.flatMap((instance) => {
    if (!instance.enabled) {
      return [];
    }

    return instance.bindings
      .filter((binding) => {
        if (!binding.enabled) {
          return false;
        }
        if (!binding.eventTypes.includes(targetEventType)) {
          return false;
        }
        return targetSeverityRank >= severityRank(binding.severityThreshold);
      })
      .map((binding) => {
        if (instance.status === "degraded") {
          return {
            instanceId: instance.id,
            instanceName: instance.name,
            cluster: binding.cluster,
            routeLabel: binding.routeLabel,
            chatId: binding.chatId,
            status: "blocked" as const,
            detail:
              "Integration is missing a valid Telegram secret reference or runtime health is degraded.",
          };
        }

        if (targetEventType === "alerts.resolved") {
          return {
            instanceId: instance.id,
            instanceName: instance.name,
            cluster: binding.cluster,
            routeLabel: binding.routeLabel,
            chatId: binding.chatId,
            status: "delivered" as const,
            detail:
              "Resolved alert delivery is enabled for this integration binding.",
          };
        }

        return {
          instanceId: instance.id,
          instanceName: instance.name,
          cluster: binding.cluster,
          routeLabel: binding.routeLabel,
          chatId: binding.chatId,
          status: "queued" as const,
          detail:
            binding.scopeType === "cluster"
              ? "Cluster-scoped Telegram routing is configured; final delivery is resolved by SERVER runtime."
              : "Global Telegram routing is configured and ready to deliver.",
        };
      });
  });

  if (projections.length > 0) {
    return projections;
  }

  return [
    {
      instanceId: "unbound",
      instanceName: "No routed integration",
      cluster: "unassigned",
      routeLabel: "manual-follow-up",
      chatId: "n/a",
      status: "not-configured",
      detail:
        "No active Telegram integration binding matched this alert event and severity.",
    },
  ];
}

export function createEmptyTelegramBindingDraft(): TelegramBindingDraft {
  return {
    scopeType: "global",
    scopeId: "",
    eventTypes: ["alerts.firing"],
    severityThreshold: "medium",
    isActive: true,
  };
}

export function createEmptyTelegramIntegrationDraft(): TelegramIntegrationDraft {
  return {
    name: "",
    description: "",
    botName: "",
    secretRef: "",
    defaultChatId: "",
    parseMode: "HTML",
    deliveryEnabled: true,
    isActive: true,
    bindings: [createEmptyTelegramBindingDraft()],
  };
}

export function toggleTelegramEventType(
  selected: TelegramBindingEventType[],
  eventType: TelegramBindingEventType
): TelegramBindingEventType[] {
  if (selected.includes(eventType)) {
    const next = selected.filter((value) => value !== eventType);
    return next.length > 0 ? next : [eventType];
  }

  return [...selected, eventType];
}

export function maskSecretRef(secretRef?: string): string {
  const trimmed = secretRef?.trim() ?? "";
  if (!trimmed) {
    return "not-set";
  }
  if (trimmed.length <= 12) {
    return "********";
  }
  return `${trimmed.slice(0, 8)}...${trimmed.slice(-4)}`;
}
