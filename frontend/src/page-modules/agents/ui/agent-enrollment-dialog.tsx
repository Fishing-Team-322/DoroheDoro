"use client";

import { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import {
  DetailGrid,
  NoticeBanner,
  StatusBadge,
  TextAreaField,
  formatMaybeValue,
} from "@/src/features/operations/ui/operations-ui";
import type { PolicySummary } from "@/src/shared/lib/runtime-api";
import { Badge, Button, Card, Input, Select } from "@/src/shared/ui";

const copyByLocale = {
  en: {
    bootstrapTokenPlaceholder:
      "Unavailable until the public Edge API bridge is exposed",
    title: "Create Agent",
    badge: "UI stub",
    description:
      "Prepare the future enrollment payload using only data already loaded in WEB. No create or enrollment request is sent from this dialog.",
    close: "Close",
    notice: {
      title: "Public Edge API bridge required",
      description:
        "Real agent create and enrollment flows will become available only after Edge exposes a public HTTP bridge for bootstrap token issuance and enrollment.",
    },
    form: {
      agentName: "Agent name",
      agentNameHelp: "UI-only draft field.",
      hostname: "Hostname",
      hostnameHelp: "Preview only. Nothing is persisted.",
      environment: "Environment",
      environmentHelp: "For example: prod, staging, lab.",
      selectPolicy: "Select policy",
      noPolicies: "No policies loaded",
      policyHelp: "Uses the policies already fetched on this page.",
      labels: "Labels / Tags (optional)",
      labelsHelp:
        "Optional preview field. Use comma-separated or newline-separated values.",
    },
    policyPreview: {
      title: "Policy preview",
      description:
        "Preview is based on the currently loaded policy list and does not call policy creation or bootstrap endpoints.",
      fields: {
        name: "Policy name",
        id: "Policy ID",
        revision: "Revision",
        status: "Status",
      },
      empty: "Load policies or select one to see the preview.",
    },
    parsedLabels: {
      title: "Parsed labels",
      empty: "No labels added yet.",
    },
    bootstrap: {
      label: "Bootstrap token",
      help: "Disabled until Edge exposes the public bridge for `agents.bootstrap-token.issue`.",
      button: "Issue Bootstrap Token",
    },
    commandPreview: {
      title: "Enrollment command preview",
      description:
        "Preview only. The command remains incomplete until a public bootstrap token bridge exists.",
    },
    footer:
      "Real enrollment and create actions stay disabled on purpose until a public Edge API bridge is available for WEB.",
    cancel: "Cancel",
    prepare: "Prepare Enrollment",
  },
  ru: {
    bootstrapTokenPlaceholder:
      "Недоступно, пока не появится публичный мост Edge API",
    title: "Создать агента",
    badge: "UI-заглушка",
    description:
      "Подготовьте будущий enrollment-payload, используя только данные, уже загруженные в WEB. Из этого диалога не отправляются запросы на создание или enrollment.",
    close: "Закрыть",
    notice: {
      title: "Нужен публичный мост Edge API",
      description:
        "Реальные сценарии создания агента и enrollment станут доступны только после того, как Edge откроет публичный HTTP-мост для выдачи bootstrap-токенов и enrollment.",
    },
    form: {
      agentName: "Имя агента",
      agentNameHelp: "Черновое поле только на уровне UI.",
      hostname: "Hostname",
      hostnameHelp: "Только предпросмотр. Ничего не сохраняется.",
      environment: "Окружение",
      environmentHelp: "Например: prod, staging, lab.",
      selectPolicy: "Выберите политику",
      noPolicies: "Политики не загружены",
      policyHelp: "Используются политики, уже загруженные на этой странице.",
      labels: "Labels / Tags (опционально)",
      labelsHelp:
        "Опциональное поле предпросмотра. Используйте значения через запятую или с новой строки.",
    },
    policyPreview: {
      title: "Предпросмотр политики",
      description:
        "Предпросмотр строится на текущем загруженном списке политик и не вызывает endpoint'ы создания политики или bootstrap.",
      fields: {
        name: "Название политики",
        id: "ID политики",
        revision: "Ревизия",
        status: "Статус",
      },
      empty: "Загрузите политики или выберите одну, чтобы увидеть предпросмотр.",
    },
    parsedLabels: {
      title: "Разобранные labels",
      empty: "Labels пока не добавлены.",
    },
    bootstrap: {
      label: "Bootstrap-токен",
      help: "Отключено, пока Edge не откроет публичный мост для `agents.bootstrap-token.issue`.",
      button: "Выдать bootstrap-токен",
    },
    commandPreview: {
      title: "Предпросмотр enrollment-команды",
      description:
        "Только предпросмотр. Команда останется неполной, пока не появится публичный мост bootstrap-токенов.",
    },
    footer:
      "Реальные действия enrollment и создания специально остаются отключенными, пока для WEB не появится публичный мост Edge API.",
    cancel: "Отмена",
    prepare: "Подготовить enrollment",
  },
} as const;

export function AgentEnrollmentDialog({
  open,
  onClose,
  policies,
  initialPolicyId,
  locale,
}: {
  open: boolean;
  onClose: () => void;
  policies: PolicySummary[];
  initialPolicyId?: string | null;
  locale: "ru" | "en";
}) {
  const copy = copyByLocale[locale];
  const [agentName, setAgentName] = useState("");
  const [hostname, setHostname] = useState("");
  const [environment, setEnvironment] = useState("");
  const [labelsText, setLabelsText] = useState("");
  const [selectedPolicyId, setSelectedPolicyId] = useState(
    initialPolicyId ?? ""
  );

  useEffect(() => {
    if (!open) {
      return;
    }

    const previousOverflow = document.body.style.overflow;
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      document.body.style.overflow = previousOverflow;
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [onClose, open]);

  const resolvedSelectedPolicyId = policies.some(
    (policy) => policy.id === selectedPolicyId
  )
    ? selectedPolicyId
    : initialPolicyId &&
        policies.some((policy) => policy.id === initialPolicyId)
      ? initialPolicyId
      : (policies[0]?.id ?? "");
  const selectedPolicy =
    policies.find((policy) => policy.id === resolvedSelectedPolicyId) ?? null;
  const labelTokens = useMemo(
    () =>
      labelsText
        .split(/\r?\n|,/)
        .map((item) => item.trim())
        .filter(Boolean),
    [labelsText]
  );
  const commandPreview = useMemo(() => {
    const lines = [
      "agentctl enroll \\",
      `  --name "${agentName.trim() || "<agent-name>"}" \\`,
      `  --hostname "${hostname.trim() || "<hostname>"}" \\`,
      `  --environment "${environment.trim() || "<environment>"}" \\`,
      ...labelTokens.map((label) => `  --label "${label}" \\`),
      `  --policy-id "${selectedPolicy?.id ?? "<policy-id>"}" \\`,
      '  --bootstrap-token "<public-edge-api-bridge-required>"',
    ];

    return lines.join("\n");
  }, [agentName, environment, hostname, labelTokens, selectedPolicy?.id]);

  if (!open || typeof document === "undefined") {
    return null;
  }

  return createPortal(
    <div
      className="fixed inset-0 z-[70] bg-black/60 p-4 backdrop-blur-[2px]"
      onClick={onClose}
    >
      <div className="flex min-h-full items-center justify-center">
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="create-agent-dialog-title"
          className="max-h-[calc(100vh-2rem)] w-full max-w-4xl overflow-y-auto"
          onClick={(event) => event.stopPropagation()}
        >
          <Card className="space-y-5 p-5 sm:p-6">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="space-y-2">
                <div className="flex flex-wrap items-center gap-2">
                  <h2
                    id="create-agent-dialog-title"
                    className="text-xl font-semibold text-[color:var(--foreground)]"
                  >
                    {copy.title}
                  </h2>
                  <Badge variant="warning">{copy.badge}</Badge>
                </div>

                <p className="max-w-3xl text-sm leading-6 text-[color:var(--muted-foreground)]">
                  {copy.description}
                </p>
              </div>

              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                onClick={onClose}
              >
                {copy.close}
              </Button>
            </div>

            <NoticeBanner
              title={copy.notice.title}
              description={copy.notice.description}
            />

            <div className="grid gap-4 md:grid-cols-2">
              <Input
                label={copy.form.agentName}
                value={agentName}
                onChange={(event) => setAgentName(event.target.value)}
                helperText={copy.form.agentNameHelp}
              />

              <Input
                label={copy.form.hostname}
                value={hostname}
                onChange={(event) => setHostname(event.target.value)}
                helperText={copy.form.hostnameHelp}
              />

              <Input
                label={copy.form.environment}
                value={environment}
                onChange={(event) => setEnvironment(event.target.value)}
                helperText={copy.form.environmentHelp}
              />

              <div className="space-y-2">
                <Select
                  id="create-agent-policy"
                  value={resolvedSelectedPolicyId}
                  onChange={(event) => setSelectedPolicyId(event.target.value)}
                  options={policies.map((policy) => ({
                    value: policy.id,
                    label: policy.name,
                  }))}
                  placeholder={
                    policies.length > 0
                      ? copy.form.selectPolicy
                      : copy.form.noPolicies
                  }
                  disabled={policies.length === 0}
                />
                <p className="text-sm text-[color:var(--muted-foreground)]">
                  {copy.form.policyHelp}
                </p>
              </div>
            </div>

            <TextAreaField
              id="create-agent-labels"
              label={copy.form.labels}
              helperText={copy.form.labelsHelp}
              value={labelsText}
              onChange={(event) => setLabelsText(event.target.value)}
              placeholder={"role=edge\nregion=eu-central-1"}
              className="min-h-24"
            />

            <div className="space-y-4 rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <div className="space-y-2">
                <p className="text-sm font-semibold text-[color:var(--foreground)]">
                  {copy.policyPreview.title}
                </p>
                <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                  {copy.policyPreview.description}
                </p>
              </div>

              {selectedPolicy ? (
                <DetailGrid
                  items={[
                    {
                      label: copy.policyPreview.fields.name,
                      value: selectedPolicy.name,
                    },
                    {
                      label: copy.policyPreview.fields.id,
                      value: selectedPolicy.id,
                    },
                    {
                      label: copy.policyPreview.fields.revision,
                      value: formatMaybeValue(selectedPolicy.revision, locale),
                    },
                    {
                      label: copy.policyPreview.fields.status,
                      value: (
                        <StatusBadge
                          value={
                            selectedPolicy.isActive === false
                              ? "inactive"
                              : "active"
                          }
                        />
                      ),
                    },
                  ]}
                />
              ) : (
                <p className="text-sm text-[color:var(--muted-foreground)]">
                  {copy.policyPreview.empty}
                </p>
              )}

              <div className="space-y-2">
                <p className="text-sm font-medium text-[color:var(--foreground)]">
                  {copy.parsedLabels.title}
                </p>
                {labelTokens.length > 0 ? (
                  <div className="flex flex-wrap gap-2">
                    {labelTokens.map((label) => (
                      <Badge key={label} variant="default">
                        {label}
                      </Badge>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    {copy.parsedLabels.empty}
                  </p>
                )}
              </div>
            </div>

            <div className="grid gap-4 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
              <div className="space-y-4 rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                <Input
                  label={copy.bootstrap.label}
                  value={copy.bootstrapTokenPlaceholder}
                  readOnly
                  disabled
                  helperText={copy.bootstrap.help}
                />

                <Button
                  variant="outline"
                  size="sm"
                  className="h-10 px-4"
                  disabled
                >
                  {copy.bootstrap.button}
                </Button>
              </div>

              <div className="space-y-3 rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                <div>
                  <p className="text-sm font-semibold text-[color:var(--foreground)]">
                    {copy.commandPreview.title}
                  </p>
                  <p className="mt-1 text-sm leading-6 text-[color:var(--muted-foreground)]">
                    {copy.commandPreview.description}
                  </p>
                </div>

                <pre className="overflow-x-auto rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] p-3 text-xs leading-6 text-[color:var(--foreground)]">
                  <code>{commandPreview}</code>
                </pre>
              </div>
            </div>

            <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[color:var(--border)] pt-4">
              <p className="max-w-3xl text-sm leading-6 text-[color:var(--muted-foreground)]">
                {copy.footer}
              </p>

              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="h-10 px-4"
                  onClick={onClose}
                >
                  {copy.cancel}
                </Button>
                <Button size="sm" className="h-10 px-4" disabled>
                  {copy.prepare}
                </Button>
              </div>
            </div>
          </Card>
        </div>
      </div>
    </div>,
    document.body
  );
}
