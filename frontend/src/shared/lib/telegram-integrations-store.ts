"use client";

export type TelegramSeverityLevel = "info" | "warning" | "critical";

export type TelegramClusterBinding = {
  id: string;
  cluster: string;
  routeLabel: string;
  chatId: string;
  severities: TelegramSeverityLevel[];
  enabled: boolean;
};

export type TelegramInstanceStatus = "active" | "paused" | "degraded";

export type TelegramConnectionStatus = "success" | "failed";

export type TelegramInstance = {
  id: string;
  name: string;
  botToken: string;
  defaultChatId: string;
  enabled: boolean;
  status: TelegramInstanceStatus;
  notes: string;
  updatedAt: string;
  lastTestAt?: string;
  lastTestStatus?: TelegramConnectionStatus;
  lastTestMessage?: string;
  bindings: TelegramClusterBinding[];
};

export type TelegramInstanceDraft = {
  id?: string;
  name: string;
  botToken: string;
  defaultChatId: string;
  enabled: boolean;
  status: TelegramInstanceStatus;
  notes: string;
  bindings: TelegramClusterBinding[];
};

export type TelegramConnectionTestResult = {
  checkedAt: string;
  latencyMs: number;
  status: TelegramConnectionStatus;
  message: string;
};

export type TelegramDeliveryProjection = {
  instanceId: string;
  instanceName: string;
  cluster: string;
  routeLabel: string;
  chatId: string;
  status: "queued" | "delivered" | "blocked" | "not-configured";
  detail: string;
};

const STORAGE_KEY = "dorohedoro.frontend.telegram.instances";

const DEFAULT_INSTANCES: TelegramInstance[] = [
  {
    id: "telegram-primary",
    name: "Primary Ops Bot",
    botToken: "750001:AA-demo-ops-primary",
    defaultChatId: "-10025001001",
    enabled: true,
    status: "active",
    notes: "Used for open alert fanout and operator acknowledgements.",
    updatedAt: "2026-03-21T19:30:00.000Z",
    lastTestAt: "2026-03-22T00:10:00.000Z",
    lastTestStatus: "success",
    lastTestMessage: "Webhook handshake succeeded and test message was accepted.",
    bindings: [
      {
        id: "binding-core",
        cluster: "core",
        routeLabel: "core-oncall",
        chatId: "-10025001001",
        severities: ["warning", "critical"],
        enabled: true,
      },
      {
        id: "binding-edge",
        cluster: "edge",
        routeLabel: "edge-ops",
        chatId: "-10025001002",
        severities: ["critical"],
        enabled: true,
      },
    ],
  },
  {
    id: "telegram-security",
    name: "Security Escalations",
    botToken: "750002:AA-demo-security",
    defaultChatId: "-10025001080",
    enabled: true,
    status: "degraded",
    notes: "Dedicated route for posture findings and high-severity anomalies.",
    updatedAt: "2026-03-21T18:45:00.000Z",
    lastTestAt: "2026-03-22T00:05:00.000Z",
    lastTestStatus: "failed",
    lastTestMessage: "Gateway answered, but the bot was not authorized for the target chat.",
    bindings: [
      {
        id: "binding-security",
        cluster: "security",
        routeLabel: "security-watch",
        chatId: "-10025001080",
        severities: ["critical"],
        enabled: true,
      },
    ],
  },
];

function cloneBinding(binding: TelegramClusterBinding): TelegramClusterBinding {
  return {
    ...binding,
    severities: [...binding.severities],
  };
}

function cloneInstance(instance: TelegramInstance): TelegramInstance {
  return {
    ...instance,
    bindings: instance.bindings.map(cloneBinding),
  };
}

function canUseStorage() {
  return typeof window !== "undefined" && typeof window.localStorage !== "undefined";
}

function normalizeBinding(binding: TelegramClusterBinding): TelegramClusterBinding {
  return {
    ...binding,
    cluster: binding.cluster.trim() || "unassigned",
    routeLabel: binding.routeLabel.trim() || "default-route",
    chatId: binding.chatId.trim(),
    severities:
      binding.severities.length > 0 ? [...binding.severities] : ["warning", "critical"],
  };
}

function normalizeInstance(instance: TelegramInstance): TelegramInstance {
  return {
    ...instance,
    name: instance.name.trim() || "Unnamed instance",
    defaultChatId: instance.defaultChatId.trim(),
    notes: instance.notes.trim(),
    bindings: instance.bindings.map(normalizeBinding),
  };
}

export function createEmptyBinding(): TelegramClusterBinding {
  return {
    id: `binding-${Date.now()}`,
    cluster: "",
    routeLabel: "",
    chatId: "",
    severities: ["warning", "critical"],
    enabled: true,
  };
}

export function createEmptyTelegramInstanceDraft(): TelegramInstanceDraft {
  return {
    name: "",
    botToken: "",
    defaultChatId: "",
    enabled: true,
    status: "active",
    notes: "",
    bindings: [createEmptyBinding()],
  };
}

export function getFallbackTelegramInstances(): TelegramInstance[] {
  return DEFAULT_INSTANCES.map(cloneInstance);
}

export function loadTelegramInstances(): TelegramInstance[] {
  if (!canUseStorage()) {
    return getFallbackTelegramInstances();
  }

  const rawValue = window.localStorage.getItem(STORAGE_KEY);
  if (!rawValue) {
    return getFallbackTelegramInstances();
  }

  try {
    const parsed = JSON.parse(rawValue) as TelegramInstance[];
    if (!Array.isArray(parsed) || parsed.length === 0) {
      return getFallbackTelegramInstances();
    }

    return parsed.map((item) => normalizeInstance(cloneInstance(item)));
  } catch {
    return getFallbackTelegramInstances();
  }
}

export function persistTelegramInstances(instances: TelegramInstance[]) {
  const normalized = instances.map((item) => normalizeInstance(cloneInstance(item)));
  if (canUseStorage()) {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(normalized));
  }
  return normalized;
}

export function upsertTelegramInstance(draft: TelegramInstanceDraft) {
  const now = new Date().toISOString();
  const instances = loadTelegramInstances();
  const current = instances.find((item) => item.id === draft.id);
  const nextInstance: TelegramInstance = normalizeInstance({
    id: draft.id ?? `telegram-${Date.now()}`,
    name: draft.name,
    botToken: draft.botToken,
    defaultChatId: draft.defaultChatId,
    enabled: draft.enabled,
    status: draft.status,
    notes: draft.notes,
    updatedAt: now,
    lastTestAt: current?.lastTestAt,
    lastTestStatus: current?.lastTestStatus,
    lastTestMessage: current?.lastTestMessage,
    bindings: draft.bindings,
  });

  const nextInstances = [
    ...instances.filter((item) => item.id !== nextInstance.id),
    nextInstance,
  ].sort((left, right) => left.name.localeCompare(right.name));

  return persistTelegramInstances(nextInstances);
}

export function deleteTelegramInstance(id: string) {
  const instances = loadTelegramInstances().filter((item) => item.id !== id);
  return persistTelegramInstances(instances);
}

export function updateTelegramTestResult(
  id: string,
  result: TelegramConnectionTestResult
) {
  const nextInstances = loadTelegramInstances().map((item) =>
    item.id === id
      ? {
          ...item,
          status: result.status === "success" ? item.status : "degraded",
          updatedAt: result.checkedAt,
          lastTestAt: result.checkedAt,
          lastTestStatus: result.status,
          lastTestMessage: result.message,
        }
      : item
  );

  return persistTelegramInstances(nextInstances);
}

function sleep(durationMs: number) {
  return new Promise((resolve) => window.setTimeout(resolve, durationMs));
}

export async function testTelegramInstanceConnection(
  draft: Pick<TelegramInstanceDraft, "botToken" | "defaultChatId" | "bindings">
): Promise<TelegramConnectionTestResult> {
  await sleep(700);

  const hasToken = draft.botToken.trim().length > 0;
  const hasChat = draft.defaultChatId.trim().length > 0;
  const hasEnabledBinding = draft.bindings.some(
    (binding) => binding.enabled && binding.chatId.trim().length > 0
  );

  if (!hasToken || !hasChat || !hasEnabledBinding) {
    return {
      checkedAt: new Date().toISOString(),
      latencyMs: 0,
      status: "failed",
      message:
        "Connection test requires a bot token, a default chat id, and at least one enabled cluster binding.",
    };
  }

  return {
    checkedAt: new Date().toISOString(),
    latencyMs: 420,
    status: "success",
    message:
      "The frontend-only adapter simulated a successful Telegram delivery handshake.",
  };
}

export function maskTelegramToken(token: string) {
  const trimmed = token.trim();
  if (trimmed.length <= 8) {
    return trimmed || "not-set";
  }

  return `${trimmed.slice(0, 4)}...${trimmed.slice(-4)}`;
}

function normalizeAlertSeverity(value?: string): TelegramSeverityLevel {
  const normalized = value?.trim().toLowerCase();
  if (normalized === "critical" || normalized === "fatal") {
    return "critical";
  }
  if (normalized === "warning" || normalized === "warn") {
    return "warning";
  }
  return "info";
}

function severityAllowed(
  binding: TelegramClusterBinding,
  severity: TelegramSeverityLevel
) {
  if (severity === "critical") {
    return binding.severities.includes("critical");
  }

  if (severity === "warning") {
    return (
      binding.severities.includes("warning") || binding.severities.includes("critical")
    );
  }

  return binding.severities.includes("info");
}

function bindingMatchesTarget(
  binding: TelegramClusterBinding,
  target: { host?: string; service?: string }
) {
  const cluster = binding.cluster.trim().toLowerCase();
  if (!cluster) {
    return true;
  }

  const host = target.host?.trim().toLowerCase() ?? "";
  const service = target.service?.trim().toLowerCase() ?? "";

  return (
    cluster === "all" ||
    host.includes(cluster) ||
    service.includes(cluster) ||
    cluster.includes(host) ||
    cluster.includes(service)
  );
}

export function projectTelegramDeliveries(
  instances: TelegramInstance[],
  input: {
    host?: string;
    service?: string;
    severity?: string;
    status?: string;
  }
): TelegramDeliveryProjection[] {
  const severity = normalizeAlertSeverity(input.severity);
  const normalizedStatus = input.status?.trim().toLowerCase() ?? "";

  const projections = instances.flatMap((instance) => {
    if (!instance.enabled) {
      return [];
    }

    const matchingBindings = instance.bindings.filter(
      (binding) =>
        binding.enabled &&
        severityAllowed(binding, severity) &&
        bindingMatchesTarget(binding, { host: input.host, service: input.service })
    );

    return matchingBindings.map((binding) => {
      let status: TelegramDeliveryProjection["status"] = "queued";
      let detail = `Route ${binding.routeLabel} is ready for ${severity} notifications.`;

      if (instance.lastTestStatus === "failed") {
        status = "blocked";
        detail = instance.lastTestMessage ?? "The last connection test failed.";
      } else if (normalizedStatus === "resolved" || normalizedStatus === "closed") {
        status = "delivered";
        detail = "Resolved alerts are marked as delivered for operator follow-up.";
      }

      return {
        instanceId: instance.id,
        instanceName: instance.name,
        cluster: binding.cluster,
        routeLabel: binding.routeLabel,
        chatId: binding.chatId || instance.defaultChatId,
        status,
        detail,
      };
    });
  });

  if (projections.length > 0) {
    return projections;
  }

  return [
    {
      instanceId: "unbound",
      instanceName: "No routed instance",
      cluster: "unassigned",
      routeLabel: "manual-follow-up",
      chatId: "n/a",
      status: "not-configured",
      detail:
        "No enabled Telegram binding matched this alert. The frontend shows a manual follow-up fallback until a backend routing contract is available.",
    },
  ];
}
