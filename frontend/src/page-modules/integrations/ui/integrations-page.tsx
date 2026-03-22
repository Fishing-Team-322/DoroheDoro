"use client";

import { useEffect, useMemo, useState } from "react";
import { translateValueLabel, useI18n } from "@/src/shared/lib/i18n";
import {
  NoticeBanner,
  SectionCard,
  formatDateTime,
} from "@/src/features/operations/ui/operations-ui";
import {
  createEmptyTelegramInstanceDraft,
  deleteTelegramInstance,
  loadTelegramInstances,
  maskTelegramToken,
  testTelegramInstanceConnection,
  updateTelegramTestResult,
  upsertTelegramInstance,
  type TelegramInstance,
  type TelegramInstanceDraft,
} from "@/src/shared/lib/telegram-integrations-store";
import { Badge, Button, Card, EmptyState, useToast } from "@/src/shared/ui";
import { TelegramInstanceForm } from "./telegram-instance-form";

const copyByLocale = {
  en: {
    validation: {
      nameRequired: "Instance name is required.",
      tokenRequired: "Bot token is required.",
      chatRequired: "Default chat id is required.",
      bindingRequired:
        "At least one enabled binding with cluster, route label, and chat id is required.",
    },
    toasts: {
      savedTitle: "Telegram instance saved",
      savedDescription:
        "The frontend-only integration adapter stored your changes for demo and manual validation flows.",
      testSuccess: "Connection test succeeded",
      testFailed: "Connection test failed",
    },
    title: "Integrations workspace",
    newInstance: "New instance",
    notice: {
      title: "Frontend-only fallback for Telegram",
      description:
        "No backend Telegram integration contract was found in the current frontend runtime layer. This page stores data locally in the browser so demos and manual operator walkthroughs stay usable without leaving frontend scope.",
    },
    metrics: {
      instances: "Instances",
      enabled: "Enabled instances",
      bindings: "Cluster bindings",
    },
    list: {
      title: "Telegram instances",
      description:
        "Pick an instance to edit it, or create a new one for a demo/manual routing setup.",
      emptyTitle: "No integration instances",
      emptyDescription:
        "Create the first Telegram instance to start mapping cluster routes.",
      bindingsSuffix: "binding(s), last test",
      notRun: "not run",
    },
    form: {
      editTitle: "Edit instance",
      createTitle: "Create instance",
      description:
        "Usable for both quick demos and manual operator data entry. Changes stay inside frontend local storage.",
      saveChanges: "Save changes",
      createInstance: "Create instance",
    },
    details: {
      title: "Selected instance detail",
      description:
        "Compact operator snapshot of the current instance status, routing coverage, and last test result.",
      name: "Name",
      lastTest: "Last test",
      bindings: "Bindings",
      testStatus: "Test status",
      notRun: "Not run",
      unknown: "Unknown",
      emptyTitle: "No instance selected",
      emptyDescription: "Select an existing instance or create a new one.",
    },
  },
  ru: {
    validation: {
      nameRequired: "Укажите имя инстанса.",
      tokenRequired: "Укажите токен бота.",
      chatRequired: "Укажите chat id по умолчанию.",
      bindingRequired:
        "Нужна хотя бы одна включенная привязка с кластером, route label и chat id.",
    },
    toasts: {
      savedTitle: "Telegram-инстанс сохранен",
      savedDescription:
        "Фронтенд-адаптер без backend сохранил изменения для demo и ручных сценариев проверки.",
      testSuccess: "Проверка подключения успешна",
      testFailed: "Проверка подключения завершилась ошибкой",
    },
    title: "Рабочее пространство интеграций",
    newInstance: "Новый инстанс",
    notice: {
      title: "Фронтенд-only fallback для Telegram",
      description:
        "В текущем frontend runtime слое не найден backend-контракт интеграции Telegram. Эта страница хранит данные локально в браузере, чтобы demo и ручные walkthrough-сценарии оставались рабочими без выхода за frontend scope.",
    },
    metrics: {
      instances: "Инстансы",
      enabled: "Включенные инстансы",
      bindings: "Привязки кластеров",
    },
    list: {
      title: "Telegram-инстансы",
      description:
        "Выберите инстанс для редактирования или создайте новый для demo/manual-маршрутизации.",
      emptyTitle: "Инстансов интеграции нет",
      emptyDescription:
        "Создайте первый Telegram-инстанс, чтобы начать маршрутизацию по кластерам.",
      bindingsSuffix: "привязок, последний тест",
      notRun: "не запускался",
    },
    form: {
      editTitle: "Редактировать инстанс",
      createTitle: "Создать инстанс",
      description:
        "Подходит и для быстрых демо, и для ручного ввода операторских данных. Изменения остаются в frontend local storage.",
      saveChanges: "Сохранить изменения",
      createInstance: "Создать инстанс",
    },
    details: {
      title: "Детали выбранного инстанса",
      description:
        "Компактная операторская сводка по текущему статусу инстанса, покрытию маршрутизации и последнему тесту.",
      name: "Имя",
      lastTest: "Последний тест",
      bindings: "Привязки",
      testStatus: "Статус теста",
      notRun: "Не запускался",
      unknown: "Неизвестно",
      emptyTitle: "Инстанс не выбран",
      emptyDescription: "Выберите существующий инстанс или создайте новый.",
    },
  },
} as const;

function toDraft(instance: TelegramInstance): TelegramInstanceDraft {
  return {
    id: instance.id,
    name: instance.name,
    botToken: instance.botToken,
    defaultChatId: instance.defaultChatId,
    enabled: instance.enabled,
    status: instance.status,
    notes: instance.notes,
    bindings: instance.bindings.map((binding) => ({
      ...binding,
      severities: [...binding.severities],
    })),
  };
}

function toBadgeVariant(status: TelegramInstance["status"]) {
  if (status === "degraded") {
    return "danger";
  }
  if (status === "paused") {
    return "warning";
  }
  return "success";
}

function validateDraft(draft: TelegramInstanceDraft, locale: "ru" | "en") {
  const copy = copyByLocale[locale];

  if (!draft.name.trim()) {
    return copy.validation.nameRequired;
  }
  if (!draft.botToken.trim()) {
    return copy.validation.tokenRequired;
  }
  if (!draft.defaultChatId.trim()) {
    return copy.validation.chatRequired;
  }
  if (
    !draft.bindings.some(
      (binding) =>
        binding.enabled &&
        binding.cluster.trim() &&
        binding.routeLabel.trim() &&
        binding.chatId.trim()
    )
  ) {
    return copy.validation.bindingRequired;
  }

  return null;
}

export function IntegrationsPage() {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
  const { showToast } = useToast();
  const [instances, setInstances] = useState<TelegramInstance[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft] = useState<TelegramInstanceDraft>(
    createEmptyTelegramInstanceDraft()
  );
  const [formError, setFormError] = useState<string | undefined>();
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);

  useEffect(() => {
    const loaded = loadTelegramInstances();
    setInstances(loaded);

    if (loaded[0]) {
      setSelectedId(loaded[0].id);
      setDraft(toDraft(loaded[0]));
    }
  }, []);

  const selectedInstance = useMemo(() => {
    return instances.find((item) => item.id === selectedId) ?? null;
  }, [instances, selectedId]);

  const activeInstances = instances.filter((item) => item.enabled).length;
  const totalBindings = instances.reduce(
    (sum, item) => sum + item.bindings.length,
    0
  );

  const selectInstance = (instance: TelegramInstance) => {
    setSelectedId(instance.id);
    setDraft(toDraft(instance));
    setFormError(undefined);
  };

  const handleCreateNew = () => {
    setSelectedId(null);
    setDraft(createEmptyTelegramInstanceDraft());
    setFormError(undefined);
  };

  const handleSave = () => {
    const error = validateDraft(draft, locale);
    if (error) {
      setFormError(error);
      return;
    }

    setSaving(true);
    try {
      const nextInstances = upsertTelegramInstance(draft);
      const saved =
        nextInstances.find((item) => item.id === draft.id) ??
        nextInstances.find(
          (item) =>
            item.name === draft.name && item.defaultChatId === draft.defaultChatId
        ) ??
        nextInstances[0];

      setInstances(nextInstances);

      if (saved) {
        setSelectedId(saved.id);
        setDraft(toDraft(saved));
      }

      setFormError(undefined);
      showToast({
        title: copy.toasts.savedTitle,
        description: copy.toasts.savedDescription,
        variant: "success",
      });
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = () => {
    if (!draft.id) {
      handleCreateNew();
      return;
    }

    const nextInstances = deleteTelegramInstance(draft.id);
    setInstances(nextInstances);

    const nextSelected = nextInstances[0] ?? null;
    if (nextSelected) {
      setSelectedId(nextSelected.id);
      setDraft(toDraft(nextSelected));
    } else {
      setSelectedId(null);
      setDraft(createEmptyTelegramInstanceDraft());
    }

    setFormError(undefined);
  };

  const handleTestConnection = async () => {
    const error = validateDraft(draft, locale);
    if (error) {
      setFormError(error);
      return;
    }

    setTesting(true);
    try {
      const result = await testTelegramInstanceConnection(draft);

      if (draft.id) {
        const nextInstances = updateTelegramTestResult(draft.id, result);
        setInstances(nextInstances);
        const updated = nextInstances.find((item) => item.id === draft.id);
        if (updated) {
          setDraft(toDraft(updated));
        }
      }

      showToast({
        title:
          result.status === "success"
            ? copy.toasts.testSuccess
            : copy.toasts.testFailed,
        description: result.message,
        variant: result.status === "success" ? "success" : "danger",
      });
      setFormError(undefined);
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="space-y-6">
          <div className="flex flex-col gap-4 border-b border-[color:var(--border)] pb-6 xl:flex-row xl:items-center xl:justify-between">
            <div className="space-y-2">
              <h2 className="text-5xl font-semibold text-[color:var(--foreground)]">
                {copy.title}
              </h2>
            </div>

            <div className="flex flex-wrap items-center gap-3">
              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                onClick={handleCreateNew}
              >
                {copy.newInstance}
              </Button>
            </div>
          </div>

          <NoticeBanner
            title={copy.notice.title}
            description={copy.notice.description}
          />

          <section className="grid gap-4 md:grid-cols-3">
            <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.instances}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {instances.length}
              </p>
            </section>

            <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.metrics.enabled}
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {activeInstances}
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
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
            <SectionCard
              title={copy.list.title}
              description={copy.list.description}
            >
              {instances.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title={copy.list.emptyTitle}
                  description={copy.list.emptyDescription}
                />
              ) : (
                <div className="space-y-3">
                  {instances.map((item) => {
                    const active = item.id === selectedId;

                    return (
                      <button
                        key={item.id}
                        type="button"
                        onClick={() => selectInstance(item)}
                        className={`w-full rounded-xl border p-4 text-left transition-colors ${
                          active
                            ? "border-[color:var(--status-info-border)] bg-[color:var(--status-info-bg)]/45"
                            : "border-[color:var(--border)] bg-[color:var(--surface)] hover:bg-[color:var(--surface-subtle)]"
                        }`}
                      >
                        <div className="flex flex-wrap items-center gap-2">
                          <Badge variant={toBadgeVariant(item.status)}>
                            {translateValueLabel(item.status, locale)}
                          </Badge>
                          <Badge>
                            {translateValueLabel(
                              item.enabled ? "enabled" : "paused",
                              locale
                            )}
                          </Badge>
                        </div>

                        <p className="mt-3 text-base font-semibold text-[color:var(--foreground)]">
                          {item.name}
                        </p>
                        <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                          {maskTelegramToken(item.botToken)} / {item.defaultChatId}
                        </p>
                        <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                          {item.bindings.length} {copy.list.bindingsSuffix}{" "}
                          {item.lastTestAt
                            ? formatDateTime(item.lastTestAt, locale)
                            : copy.list.notRun}
                        </p>
                      </button>
                    );
                  })}
                </div>
              )}
            </SectionCard>

            <SectionCard
              title={draft.id ? copy.form.editTitle : copy.form.createTitle}
              description={copy.form.description}
            >
              <TelegramInstanceForm
                draft={draft}
                onChange={setDraft}
                onSubmit={handleSave}
                onTestConnection={() => void handleTestConnection()}
                onDelete={handleDelete}
                formError={formError}
                saveLabel={draft.id ? copy.form.saveChanges : copy.form.createInstance}
                testing={testing}
                saving={saving}
                deletable={Boolean(draft.id)}
              />
            </SectionCard>
          </section>

          <SectionCard
            title={copy.details.title}
            description={copy.details.description}
          >
            {selectedInstance ? (
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                <Card className="space-y-2 p-4">
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    {copy.details.name}
                  </p>
                  <p className="text-base font-semibold text-[color:var(--foreground)]">
                    {selectedInstance.name}
                  </p>
                </Card>

                <Card className="space-y-2 p-4">
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    {copy.details.lastTest}
                  </p>
                  <p className="text-base font-semibold text-[color:var(--foreground)]">
                    {selectedInstance.lastTestAt
                      ? formatDateTime(selectedInstance.lastTestAt, locale)
                      : copy.details.notRun}
                  </p>
                </Card>

                <Card className="space-y-2 p-4">
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    {copy.details.bindings}
                  </p>
                  <p className="text-base font-semibold text-[color:var(--foreground)]">
                    {selectedInstance.bindings.length}
                  </p>
                </Card>

                <Card className="space-y-2 p-4">
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    {copy.details.testStatus}
                  </p>
                  <p className="text-base font-semibold text-[color:var(--foreground)]">
                    {selectedInstance.lastTestStatus
                      ? translateValueLabel(selectedInstance.lastTestStatus, locale)
                      : copy.details.unknown}
                  </p>
                </Card>
              </div>
            ) : (
              <EmptyState
                variant="flush"
                title={copy.details.emptyTitle}
                description={copy.details.emptyDescription}
              />
            )}
          </SectionCard>
        </div>
      </Card>
    </div>
  );
}
