"use client";

import { useEffect, useMemo, useState } from "react";
import { translateValueLabel, useI18n } from "@/src/shared/lib/i18n";
import {
  NoticeBanner,
  SectionCard,
  formatDateTime,
} from "@/src/features/operations/ui/operations-ui";
import { isApiError } from "@/src/shared/lib/api";
import {
  createEmptyTelegramIntegrationDraft,
  type TelegramIntegrationDraft,
  type TelegramRuntimeEvent,
} from "@/src/shared/lib/telegram-integrations";
import {
  createIntegration,
  createIntegrationBinding,
  deleteIntegrationBinding,
  getIntegration,
  listClusters,
  listIntegrations,
  requestTelegramIntegrationHealthcheck,
  updateIntegration,
  type ClusterItem,
  type IntegrationBindingPayload,
  type IntegrationItem,
} from "@/src/shared/lib/runtime-api";
import { Badge, Button, Card, EmptyState, useToast } from "@/src/shared/ui";
import { TelegramInstanceForm } from "./telegram-instance-form";

const copyByLocale = {
  en: {
    title: "Telegram orchestration",
    newItem: "New integration",
    refresh: "Refresh",
    noticeTitle: "Live backend workspace",
    noticeDescription:
      "This page now works against the real integrations API and Telegram runtime. Healthcheck results stream back live from SERVER.",
    metrics: {
      instances: "Telegram bots",
      enabled: "Active",
      bindings: "Bindings",
      stream: "Stream",
    },
    stream: {
      connecting: "Connecting",
      open: "Connected",
      closed: "Closed",
      error: "Error",
    },
    listTitle: "Telegram integrations",
    listDescription:
      "Pick a bot integration to edit bindings, default chat, and runtime state.",
    emptyTitle: "No Telegram integrations",
    emptyDescription: "Create the first Telegram integration.",
    bindingsSuffix: "binding(s)",
    lastActivity: "Last activity",
    none: "none yet",
    formEdit: "Edit Telegram integration",
    formCreate: "Create Telegram integration",
    formDescription:
      "The form follows the live backend contract: Vault ref, default chat, delivery toggle, scope bindings, event types, severity.",
    saveChanges: "Save changes",
    createItem: "Create integration",
    detailsTitle: "Runtime detail",
    detailsDescription:
      "Current runtime state plus recent delivery and healthcheck events.",
    noSelection: "Select an integration to inspect runtime detail.",
    activityTitle: "Live activity",
    activityEmpty: "No Telegram runtime events yet.",
    validation: {
      nameRequired: "Integration name is required.",
      secretRefRequired:
        "Vault secret ref is required when creating a Telegram integration.",
      clusterRequired: "Each cluster-scoped binding requires a cluster.",
      eventTypesRequired: "Each binding must include at least one event type.",
    },
    toast: {
      savedTitle: "Telegram integration saved",
      savedDescription: "Changes were written to the live SERVER runtime.",
      healthcheckQueued: "Telegram healthcheck queued",
      healthcheckSuccess: "Telegram healthcheck succeeded",
      healthcheckFailed: "Telegram healthcheck failed",
    },
  },
  ru: {
    title: "Telegram-оркестрация",
    newItem: "Новая интеграция",
    refresh: "Обновить",
    noticeTitle: "Живое backend-рабочее пространство",
    noticeDescription:
      "Страница теперь работает через реальный integrations API и Telegram runtime. Результаты healthcheck приходят обратно live из SERVER.",
    metrics: {
      instances: "Telegram-боты",
      enabled: "Активные",
      bindings: "Привязки",
      stream: "Stream",
    },
    stream: {
      connecting: "Подключение",
      open: "Подключён",
      closed: "Закрыт",
      error: "Ошибка",
    },
    listTitle: "Telegram-интеграции",
    listDescription:
      "Выберите интеграцию бота, чтобы менять bindings, default chat и runtime state.",
    emptyTitle: "Telegram-интеграций нет",
    emptyDescription: "Создайте первую Telegram-интеграцию.",
    bindingsSuffix: "binding(s)",
    lastActivity: "Последняя активность",
    none: "пока нет",
    formEdit: "Редактировать Telegram-интеграцию",
    formCreate: "Создать Telegram-интеграцию",
    formDescription:
      "Форма совпадает с живым backend-контрактом: Vault ref, default chat, delivery toggle, scope bindings, event types, severity.",
    saveChanges: "Сохранить изменения",
    createItem: "Создать интеграцию",
    detailsTitle: "Runtime detail",
    detailsDescription:
      "Текущее runtime-состояние и недавние delivery/healthcheck события.",
    noSelection: "Выберите интеграцию, чтобы увидеть runtime detail.",
    activityTitle: "Live activity",
    activityEmpty: "Для этой интеграции пока нет Telegram runtime events.",
    validation: {
      nameRequired: "Укажите имя интеграции.",
      secretRefRequired:
        "При создании Telegram-интеграции нужен Vault secret ref.",
      clusterRequired: "Для cluster-scoped binding нужно выбрать кластер.",
      eventTypesRequired:
        "В каждой binding должен быть хотя бы один тип события.",
    },
    toast: {
      savedTitle: "Telegram-интеграция сохранена",
      savedDescription: "Изменения записаны в живой SERVER runtime.",
      healthcheckQueued: "Telegram healthcheck поставлен в очередь",
      healthcheckSuccess: "Telegram healthcheck успешен",
      healthcheckFailed: "Telegram healthcheck завершился ошибкой",
    },
  },
} as const;

type StreamState = "connecting" | "open" | "closed" | "error";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function asString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value : undefined;
}

function mapIntegrationToDraft(
  item: IntegrationItem
): TelegramIntegrationDraft {
  const config = isRecord(item.config_json) ? item.config_json : {};

  return {
    id: item.integration_id,
    name: item.name,
    description: item.description ?? "",
    botName: asString(config.bot_name) ?? item.name ?? "telegram-bot",
    secretRef: "",
    maskedSecretRef: asString(config.masked_secret_ref),
    hasSecretRef:
      typeof config.has_secret_ref === "boolean"
        ? config.has_secret_ref
        : undefined,
    defaultChatId: asString(config.default_chat_id) ?? "",
    parseMode:
      (asString(config.parse_mode) as TelegramIntegrationDraft["parseMode"]) ??
      "HTML",
    deliveryEnabled:
      typeof config.delivery_enabled === "boolean"
        ? config.delivery_enabled
        : true,
    isActive: item.is_active,
    bindings:
      item.bindings.length > 0
        ? item.bindings.map((binding) => ({
            id: binding.integration_binding_id,
            scopeType: binding.scope_type === "cluster" ? "cluster" : "global",
            scopeId: binding.scope_id ?? "",
            eventTypes:
              binding.event_types_json.length > 0
                ? (binding.event_types_json as TelegramIntegrationDraft["bindings"][number]["eventTypes"])
                : ["alerts.firing"],
            severityThreshold:
              (binding.severity_threshold as TelegramIntegrationDraft["bindings"][number]["severityThreshold"]) ??
              "medium",
            isActive: binding.is_active,
          }))
        : createEmptyTelegramIntegrationDraft().bindings,
  };
}

function draftToIntegrationPayload(
  draft: TelegramIntegrationDraft,
  reason: string
) {
  const configJson: Record<string, unknown> = {
    bot_name: draft.botName.trim() || draft.name.trim(),
    parse_mode: draft.parseMode,
    message_template_version: "v1",
    delivery_enabled: draft.deliveryEnabled,
  };

  if (draft.defaultChatId.trim()) {
    configJson.default_chat_id = draft.defaultChatId.trim();
  }
  if (draft.secretRef.trim()) {
    configJson.secret_ref = draft.secretRef.trim();
  }

  return {
    name: draft.name.trim(),
    kind: "telegram_bot",
    description: draft.description.trim(),
    config_json: configJson,
    is_active: draft.isActive,
    reason,
  };
}

function draftToBindingPayloads(
  draft: TelegramIntegrationDraft
): IntegrationBindingPayload[] {
  return draft.bindings.map((binding) => ({
    scope_type: binding.scopeType,
    scope_id: binding.scopeType === "cluster" ? binding.scopeId : "",
    event_types_json: binding.eventTypes,
    severity_threshold: binding.severityThreshold,
    is_active: binding.isActive,
    reason: "website telegram binding sync",
  }));
}

function parseRuntimeEvent(
  event: string,
  rawMessage: string
): TelegramRuntimeEvent | null {
  try {
    const parsed = JSON.parse(rawMessage) as unknown;
    if (!isRecord(parsed)) {
      return null;
    }
    const status = isRecord(parsed.status) ? parsed.status : {};
    const requestId =
      asString(parsed.request_id) ?? asString(status.correlation_id);
    const deliveryId = asString(parsed.delivery_id);

    return {
      id: `${event}:${requestId ?? deliveryId ?? Date.now().toString()}`,
      event,
      integrationId: asString(parsed.integration_id),
      requestId,
      deliveryStatus: asString(parsed.delivery_status),
      classification: asString(parsed.classification),
      messageId: asString(parsed.telegram_message_id),
      statusCode: asString(status.code),
      statusMessage: asString(status.message),
      createdAt: asString(parsed.created_at) ?? asString(status.created_at),
      raw: parsed,
    };
  } catch {
    return null;
  }
}

function describeEvent(
  event: TelegramRuntimeEvent,
  locale: "ru" | "en"
): string {
  if (event.event === "telegram-healthcheck-result") {
    return locale === "ru" ? "Healthcheck" : "Healthcheck";
  }
  if (event.event === "telegram-delivery-queued") {
    return locale === "ru" ? "Поставлено в очередь" : "Queued";
  }
  if (event.event === "telegram-delivery-succeeded") {
    return locale === "ru" ? "Доставлено" : "Delivered";
  }
  if (event.event === "telegram-delivery-failed") {
    return locale === "ru" ? "Ошибка доставки" : "Failed";
  }
  return event.event;
}

function toBadgeVariant(item: IntegrationItem, events: TelegramRuntimeEvent[]) {
  const latest = events[0];
  if (latest?.event === "telegram-healthcheck-result") {
    if (latest.deliveryStatus === "delivered") {
      return "success" as const;
    }
    if (latest.deliveryStatus === "failed") {
      return "danger" as const;
    }
  }

  const config = isRecord(item.config_json) ? item.config_json : {};
  if (!item.is_active || config.delivery_enabled === false) {
    return "warning" as const;
  }
  return "success" as const;
}

export function IntegrationsPage() {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
  const { showToast } = useToast();
  const [integrations, setIntegrations] = useState<IntegrationItem[]>([]);
  const [clusters, setClusters] = useState<ClusterItem[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft] = useState<TelegramIntegrationDraft>(
    createEmptyTelegramIntegrationDraft()
  );
  const [formError, setFormError] = useState<string | undefined>();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [streamState, setStreamState] = useState<StreamState>("connecting");
  const [activityByIntegration, setActivityByIntegration] = useState<
    Record<string, TelegramRuntimeEvent[]>
  >({});
  const [pendingHealthcheckRequestId, setPendingHealthcheckRequestId] =
    useState<string | null>(null);

  const selectedIntegration = useMemo(
    () =>
      integrations.find((item) => item.integration_id === selectedId) ?? null,
    [integrations, selectedId]
  );
  const selectedEvents = useMemo(
    () => (selectedId ? (activityByIntegration[selectedId] ?? []) : []),
    [activityByIntegration, selectedId]
  );

  const loadDetail = async (integrationId: string) => {
    const response = await getIntegration(integrationId);
    setDraft(mapIntegrationToDraft(response.item));
  };

  const loadPage = async (preferredId?: string | null) => {
    setLoading(true);
    try {
      const [integrationsResponse, clustersResponse] = await Promise.all([
        listIntegrations({ limit: 100, offset: 0 }),
        listClusters({ limit: 100, offset: 0 }),
      ]);
      const telegramIntegrations = integrationsResponse.items.filter(
        (item) => item.kind === "telegram_bot"
      );
      setIntegrations(telegramIntegrations);
      setClusters(clustersResponse.items);

      const nextId =
        preferredId &&
        telegramIntegrations.some((item) => item.integration_id === preferredId)
          ? preferredId
          : (telegramIntegrations[0]?.integration_id ?? null);
      setSelectedId(nextId);
      if (nextId) {
        await loadDetail(nextId);
      } else {
        setDraft(createEmptyTelegramIntegrationDraft());
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void loadPage(selectedId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (!selectedId) {
      setStreamState("closed");
      return;
    }

    setStreamState("connecting");
    const source = new EventSource(
      `/api/edge/api/v1/stream/integrations?integration_id=${encodeURIComponent(selectedId)}`
    );

    const pushEvent =
      (eventName: string) => (message: MessageEvent<string>) => {
        const parsed = parseRuntimeEvent(eventName, message.data);
        if (!parsed?.integrationId) {
          return;
        }

        setActivityByIntegration((current) => ({
          ...current,
          [parsed.integrationId!]: [
            parsed,
            ...(current[parsed.integrationId!] ?? []),
          ].slice(0, 20),
        }));

        if (
          eventName === "telegram-healthcheck-result" &&
          parsed.requestId &&
          parsed.requestId === pendingHealthcheckRequestId
        ) {
          showToast({
            title:
              parsed.deliveryStatus === "delivered"
                ? copy.toast.healthcheckSuccess
                : copy.toast.healthcheckFailed,
            description:
              parsed.statusMessage ??
              parsed.statusCode ??
              parsed.deliveryStatus ??
              "",
            variant:
              parsed.deliveryStatus === "delivered" ? "success" : "danger",
          });
          setPendingHealthcheckRequestId(null);
        }
      };

    source.addEventListener("ready", () => setStreamState("open"));
    source.addEventListener(
      "telegram-delivery-queued",
      pushEvent("telegram-delivery-queued") as EventListener
    );
    source.addEventListener(
      "telegram-delivery-succeeded",
      pushEvent("telegram-delivery-succeeded") as EventListener
    );
    source.addEventListener(
      "telegram-delivery-failed",
      pushEvent("telegram-delivery-failed") as EventListener
    );
    source.addEventListener(
      "telegram-healthcheck-result",
      pushEvent("telegram-healthcheck-result") as EventListener
    );
    source.onerror = () => setStreamState("error");

    return () => {
      source.close();
      setStreamState("closed");
    };
  }, [
    copy.toast.healthcheckFailed,
    copy.toast.healthcheckSuccess,
    pendingHealthcheckRequestId,
    selectedId,
    showToast,
  ]);

  const validateDraft = (
    current: TelegramIntegrationDraft,
    requireSecretRef: boolean
  ) => {
    if (!current.name.trim()) {
      return copy.validation.nameRequired;
    }
    if (requireSecretRef && !current.secretRef.trim()) {
      return copy.validation.secretRefRequired;
    }
    for (const binding of current.bindings) {
      if (binding.scopeType === "cluster" && !binding.scopeId.trim()) {
        return copy.validation.clusterRequired;
      }
      if (binding.eventTypes.length === 0) {
        return copy.validation.eventTypesRequired;
      }
    }
    return null;
  };

  const syncBindings = async (
    integrationId: string,
    nextDraft: TelegramIntegrationDraft
  ) => {
    const detail = await getIntegration(integrationId);
    for (const binding of detail.item.bindings) {
      await deleteIntegrationBinding(
        integrationId,
        binding.integration_binding_id
      );
    }
    for (const payload of draftToBindingPayloads(nextDraft)) {
      await createIntegrationBinding(integrationId, payload);
    }
  };

  const persistDraft = async (nextDraft: TelegramIntegrationDraft) => {
    const validationError = validateDraft(nextDraft, !nextDraft.id);
    if (validationError) {
      setFormError(validationError);
      return null;
    }

    setSaving(true);
    try {
      let integrationId = nextDraft.id ?? null;
      if (integrationId) {
        await updateIntegration(
          integrationId,
          draftToIntegrationPayload(
            nextDraft,
            "website telegram integration updated"
          )
        );
      } else {
        const response = await createIntegration(
          draftToIntegrationPayload(
            nextDraft,
            "website telegram integration created"
          )
        );
        integrationId = response.data.item.integration_id;
      }

      if (!integrationId) {
        return null;
      }

      await syncBindings(integrationId, nextDraft);
      await loadPage(integrationId);
      setFormError(undefined);
      showToast({
        title: copy.toast.savedTitle,
        description: copy.toast.savedDescription,
        variant: "success",
      });
      return integrationId;
    } catch (error) {
      setFormError(
        isApiError(error)
          ? error.message
          : error instanceof Error
            ? error.message
            : "Request failed"
      );
      return null;
    } finally {
      setSaving(false);
    }
  };

  const handleTestConnection = async () => {
    setTesting(true);
    try {
      const integrationId = draft.id ?? (await persistDraft(draft));
      if (!integrationId) {
        return;
      }

      const response = await requestTelegramIntegrationHealthcheck(
        integrationId,
        {
          reason: "website telegram healthcheck requested",
        }
      );
      setPendingHealthcheckRequestId(response.data.request_id);
      showToast({
        title: copy.toast.healthcheckQueued,
        description: response.data.request_id,
        variant: "success",
      });
    } catch (error) {
      setFormError(
        isApiError(error)
          ? error.message
          : error instanceof Error
            ? error.message
            : copy.toast.healthcheckFailed
      );
    } finally {
      setTesting(false);
    }
  };

  const activeIntegrations = integrations.filter(
    (item) => item.is_active
  ).length;
  const totalBindings = integrations.reduce(
    (sum, item) => sum + item.bindings.length,
    0
  );
  const latestSelectedEvent = selectedEvents[0];

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="space-y-6">
          <div className="flex flex-col gap-4 border-b border-[color:var(--border)] pb-6 xl:flex-row xl:items-center xl:justify-between">
            <h2 className="text-5xl font-semibold text-[color:var(--foreground)]">
              {copy.title}
            </h2>

            <div className="flex flex-wrap items-center gap-3">
              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                onClick={() => void loadPage(selectedId)}
              >
                {copy.refresh}
              </Button>
              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                onClick={() => {
                  setSelectedId(null);
                  setDraft(createEmptyTelegramIntegrationDraft());
                  setFormError(undefined);
                }}
              >
                {copy.newItem}
              </Button>
            </div>
          </div>

          <NoticeBanner
            title={copy.noticeTitle}
            description={copy.noticeDescription}
          />

          <section className="grid gap-4 md:grid-cols-4">
            <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.instances}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {integrations.length}
              </p>
            </section>
            <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.enabled}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {activeIntegrations}
              </p>
            </section>
            <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.bindings}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {totalBindings}
              </p>
            </section>
            <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.stream}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {copy.stream[streamState]}
              </p>
            </section>
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
            <SectionCard
              title={copy.listTitle}
              description={copy.listDescription}
            >
              {loading ? (
                <p className="text-sm text-[color:var(--muted-foreground)]">
                  Loading...
                </p>
              ) : integrations.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title={copy.emptyTitle}
                  description={copy.emptyDescription}
                />
              ) : (
                <div className="space-y-3">
                  {integrations.map((item) => {
                    const active = item.integration_id === selectedId;
                    const events =
                      activityByIntegration[item.integration_id] ?? [];
                    const config = isRecord(item.config_json)
                      ? item.config_json
                      : {};

                    return (
                      <button
                        key={item.integration_id}
                        type="button"
                        onClick={() => {
                          setSelectedId(item.integration_id);
                          setFormError(undefined);
                          void loadDetail(item.integration_id);
                        }}
                        className={`w-full rounded-xl border p-4 text-left transition-colors ${
                          active
                            ? "border-[color:var(--status-info-border)] bg-[color:var(--status-info-bg)]/45"
                            : "border-[color:var(--border)] bg-[color:var(--surface)] hover:bg-[color:var(--surface-subtle)]"
                        }`}
                      >
                        <div className="flex flex-wrap items-center gap-2">
                          <Badge variant={toBadgeVariant(item, events)}>
                            {translateValueLabel(
                              item.is_active ? "active" : "paused",
                              locale
                            )}
                          </Badge>
                          <Badge>
                            {config.delivery_enabled === false
                              ? locale === "ru"
                                ? "доставка на паузе"
                                : "delivery paused"
                              : locale === "ru"
                                ? "доставка включена"
                                : "delivery enabled"}
                          </Badge>
                        </div>

                        <p className="mt-3 text-base font-semibold text-[color:var(--foreground)]">
                          {item.name}
                        </p>
                        <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                          {asString(config.default_chat_id) ?? "n/a"} /{" "}
                          {asString(config.masked_secret_ref) ?? "n/a"}
                        </p>
                        <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                          {item.bindings.length} {copy.bindingsSuffix} /{" "}
                          {copy.lastActivity}{" "}
                          {events[0]?.createdAt
                            ? formatDateTime(events[0].createdAt, locale)
                            : copy.none}
                        </p>
                      </button>
                    );
                  })}
                </div>
              )}
            </SectionCard>

            <SectionCard
              title={draft.id ? copy.formEdit : copy.formCreate}
              description={copy.formDescription}
            >
              <TelegramInstanceForm
                draft={draft}
                clusters={clusters}
                onChange={setDraft}
                onSubmit={() => void persistDraft(draft)}
                onTestConnection={() => void handleTestConnection()}
                formError={formError}
                saveLabel={draft.id ? copy.saveChanges : copy.createItem}
                testing={testing}
                saving={saving}
                deletable={false}
              />
            </SectionCard>
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)]">
            <SectionCard
              title={copy.detailsTitle}
              description={copy.detailsDescription}
            >
              {selectedIntegration ? (
                <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                  <Card className="space-y-2 p-4">
                    <p className="text-sm text-[color:var(--muted-foreground)]">
                      Bot
                    </p>
                    <p className="text-base font-semibold text-[color:var(--foreground)]">
                      {draft.botName || selectedIntegration.name}
                    </p>
                  </Card>
                  <Card className="space-y-2 p-4">
                    <p className="text-sm text-[color:var(--muted-foreground)]">
                      Chat
                    </p>
                    <p className="text-base font-semibold text-[color:var(--foreground)]">
                      {draft.defaultChatId || "n/a"}
                    </p>
                  </Card>
                  <Card className="space-y-2 p-4">
                    <p className="text-sm text-[color:var(--muted-foreground)]">
                      Delivery
                    </p>
                    <p className="text-base font-semibold text-[color:var(--foreground)]">
                      {draft.deliveryEnabled
                        ? locale === "ru"
                          ? "включена"
                          : "enabled"
                        : locale === "ru"
                          ? "на паузе"
                          : "paused"}
                    </p>
                  </Card>
                  <Card className="space-y-2 p-4">
                    <p className="text-sm text-[color:var(--muted-foreground)]">
                      {copy.lastActivity}
                    </p>
                    <p className="text-base font-semibold text-[color:var(--foreground)]">
                      {latestSelectedEvent?.createdAt
                        ? formatDateTime(latestSelectedEvent.createdAt, locale)
                        : copy.none}
                    </p>
                  </Card>
                </div>
              ) : (
                <EmptyState
                  variant="flush"
                  title={copy.detailsTitle}
                  description={copy.noSelection}
                />
              )}
            </SectionCard>

            <SectionCard
              title={copy.activityTitle}
              description={copy.detailsDescription}
            >
              {selectedEvents.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title={copy.activityTitle}
                  description={copy.activityEmpty}
                />
              ) : (
                <div className="space-y-3">
                  {selectedEvents.map((event) => (
                    <Card key={event.id} className="space-y-2 p-4">
                      <div className="flex flex-wrap items-center gap-2">
                        <Badge
                          variant={
                            event.deliveryStatus === "delivered"
                              ? "success"
                              : event.deliveryStatus === "failed"
                                ? "danger"
                                : "default"
                          }
                        >
                          {describeEvent(event, locale)}
                        </Badge>
                        {event.statusCode ? (
                          <Badge>{event.statusCode}</Badge>
                        ) : null}
                      </div>
                      <p className="text-sm font-semibold text-[color:var(--foreground)]">
                        {event.statusMessage ??
                          event.classification ??
                          event.event}
                      </p>
                      <p className="text-xs text-[color:var(--muted-foreground)]">
                        {event.createdAt
                          ? formatDateTime(event.createdAt, locale)
                          : copy.none}
                        {event.messageId ? ` / message ${event.messageId}` : ""}
                        {event.requestId ? ` / req ${event.requestId}` : ""}
                      </p>
                    </Card>
                  ))}
                </div>
              )}
            </SectionCard>
          </section>
        </div>
      </Card>
    </div>
  );
}
