"use client";

import { useEffect, useMemo, useState } from "react";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { useI18n } from "@/src/shared/lib/i18n";
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

function validateDraft(draft: TelegramInstanceDraft) {
  if (!draft.name.trim()) {
    return "Instance name is required.";
  }
  if (!draft.botToken.trim()) {
    return "Bot token is required.";
  }
  if (!draft.defaultChatId.trim()) {
    return "Default chat id is required.";
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
    return "At least one enabled binding with cluster, route label, and chat id is required.";
  }

  return null;
}

export function IntegrationsPage() {
  const { dictionary } = useI18n();
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
    const error = validateDraft(draft);
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
        title: "Telegram instance saved",
        description:
          "The frontend-only integration adapter stored your changes for demo and manual validation flows.",
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
    const error = validateDraft(draft);
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
            ? "Connection test succeeded"
            : "Connection test failed",
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
                integrations workspace
              </h2>
            </div>

            <div className="flex flex-wrap items-center gap-3">
              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                onClick={handleCreateNew}
              >
                New instance
              </Button>
            </div>
          </div>

          <NoticeBanner
            title="Frontend-only fallback for Telegram"
            description="No backend Telegram integration contract was found in the current frontend runtime layer. This page stores data locally in the browser so demos and manual operator walkthroughs stay usable without leaving frontend scope."
          />

          <section className="grid gap-4 md:grid-cols-3">
            <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Instances
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {instances.length}
              </p>
            </section>

            <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Enabled instances
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {activeInstances}
              </p>
            </section>

            <section className="space-y-2 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Cluster bindings
              </p>
              <p className="text-3xl font-semibold text-[color:var(--foreground)]">
                {totalBindings}
              </p>
            </section>
          </section>

          <section className="grid gap-4 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
            <SectionCard
              title="Telegram instances"
              description="Pick an instance to edit it, or create a new one for a demo/manual routing setup."
            >
              {instances.length === 0 ? (
                <EmptyState
                  variant="flush"
                  title="No integration instances"
                  description="Create the first Telegram instance to start mapping cluster routes."
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
                            {item.status}
                          </Badge>
                          <Badge>{item.enabled ? "enabled" : "paused"}</Badge>
                        </div>

                        <p className="mt-3 text-base font-semibold text-[color:var(--foreground)]">
                          {item.name}
                        </p>
                        <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                          {maskTelegramToken(item.botToken)} / {item.defaultChatId}
                        </p>
                        <p className="mt-2 text-sm text-[color:var(--muted-foreground)]">
                          {item.bindings.length} binding(s), last test{" "}
                          {item.lastTestAt
                            ? formatDateTime(item.lastTestAt)
                            : "not run"}
                        </p>
                      </button>
                    );
                  })}
                </div>
              )}
            </SectionCard>

            <SectionCard
              title={draft.id ? "Edit instance" : "Create instance"}
              description="Usable for both quick demos and manual operator data entry. Changes stay inside frontend local storage."
            >
              <TelegramInstanceForm
                draft={draft}
                onChange={setDraft}
                onSubmit={handleSave}
                onTestConnection={() => void handleTestConnection()}
                onDelete={handleDelete}
                formError={formError}
                saveLabel={draft.id ? "Save changes" : "Create instance"}
                testing={testing}
                saving={saving}
                deletable={Boolean(draft.id)}
              />
            </SectionCard>
          </section>

          <SectionCard
            title="Selected instance detail"
            description="Compact operator snapshot of the current instance status, routing coverage, and last test result."
          >
            {selectedInstance ? (
              <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                <Card className="space-y-2 p-4">
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    Name
                  </p>
                  <p className="text-base font-semibold text-[color:var(--foreground)]">
                    {selectedInstance.name}
                  </p>
                </Card>

                <Card className="space-y-2 p-4">
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    Last test
                  </p>
                  <p className="text-base font-semibold text-[color:var(--foreground)]">
                    {selectedInstance.lastTestAt
                      ? formatDateTime(selectedInstance.lastTestAt)
                      : "Not run"}
                  </p>
                </Card>

                <Card className="space-y-2 p-4">
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    Bindings
                  </p>
                  <p className="text-base font-semibold text-[color:var(--foreground)]">
                    {selectedInstance.bindings.length}
                  </p>
                </Card>

                <Card className="space-y-2 p-4">
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    Test status
                  </p>
                  <p className="text-base font-semibold text-[color:var(--foreground)]">
                    {selectedInstance.lastTestStatus ?? "unknown"}
                  </p>
                </Card>
              </div>
            ) : (
              <EmptyState
                variant="flush"
                title="No instance selected"
                description="Select an existing instance or create a new one."
              />
            )}
          </SectionCard>
        </div>
      </Card>
    </div>
  );
}