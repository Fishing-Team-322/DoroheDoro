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
import {
  getPolicyRevisions,
  issueBootstrapToken,
  type PolicySummary,
} from "@/src/shared/lib/runtime-api";
import { Badge, Button, Card, Input, Select, useToast } from "@/src/shared/ui";

const copyByLocale = {
  en: {
    bootstrapTokenPlaceholder:
      "Issue a bootstrap token for the selected policy",
    title: "Create Agent",
    badge: "Edge API",
    description:
      "Prepare enrollment parameters, issue a bootstrap token through the public Edge API, and get a working agent enrollment command.",
    close: "Close",
    notice: {
      title: "Public Edge API bridge is live",
      description:
        "This dialog now uses the live public Edge API bridge: it resolves the policy revision, issues a bootstrap token, and updates the enrollment command without manual host edits.",
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
        "The selected policy comes from the live Edge API. Bootstrap issuance resolves the current policy revision before requesting a token.",
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
      help: "Issued through `POST /api/v1/agents/bootstrap-tokens`.",
      button: "Issue Bootstrap Token",
    },
    commandPreview: {
      title: "Enrollment command preview",
      description:
        "Once a bootstrap token is issued, this command is updated automatically and becomes ready for a real enrollment.",
    },
    footer:
      "Issue a bootstrap token, then use the prepared enrollment command or the deployment workflow on the main page.",
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
  const { showToast } = useToast();
  const liveBadge = locale === "ru" ? "Edge API" : "Edge API";
  const description =
    locale === "ru"
      ? "Соберите enrollment-параметры, выпустите bootstrap-токен через публичный Edge API и получите рабочую команду для запуска агента."
      : "Prepare enrollment parameters, issue a bootstrap token through the public Edge API, and get a working agent enrollment command.";
  const noticeTitle =
    locale === "ru"
      ? "Публичный мост Edge API активен"
      : "Public Edge API bridge is live";
  const noticeDescription =
    locale === "ru"
      ? "Диалог использует живой публичный мост Edge API: получает ревизию policy, выпускает bootstrap-токен и обновляет enrollment-команду без ручных правок на хостах."
      : "This dialog now uses the live public Edge API bridge: it resolves the policy revision, issues a bootstrap token, and updates the enrollment command without manual host edits.";
  const commandDescription =
    locale === "ru"
      ? "После выдачи bootstrap-токена команда автоматически обновится и станет пригодной для реального enrollment."
      : "Once a bootstrap token is issued, this command is updated automatically and becomes ready for a real enrollment.";
  const bootstrapTokenPlaceholder =
    locale === "ru"
      ? "Выдайте bootstrap-токен для выбранной policy"
      : "Issue a bootstrap token for the selected policy";
  const footerText =
    locale === "ru"
      ? "Выдайте bootstrap-токен, затем используйте готовую enrollment-команду или deployment workflow на основной странице."
      : "Issue a bootstrap token, then use the prepared enrollment command or the deployment workflow on the main page.";
  const [agentName, setAgentName] = useState("");
  const [hostname, setHostname] = useState("");
  const [environment, setEnvironment] = useState("");
  const [labelsText, setLabelsText] = useState("");
  const [selectedPolicyId, setSelectedPolicyId] = useState(
    initialPolicyId ?? ""
  );
  const [bootstrapLoading, setBootstrapLoading] = useState(false);
  const [bootstrapError, setBootstrapError] = useState<string>();
  const [bootstrapTokenValue, setBootstrapTokenValue] = useState("");
  const [bootstrapExpiresAtUnixMs, setBootstrapExpiresAtUnixMs] = useState<
    number | undefined
  >();

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
      `  --bootstrap-token "${bootstrapTokenValue || "<issue-bootstrap-token>"}"`,
    ];

    return lines.join("\n");
  }, [
    agentName,
    bootstrapTokenValue,
    environment,
    hostname,
    labelTokens,
    selectedPolicy?.id,
  ]);

  useEffect(() => {
    setBootstrapError(undefined);
    setBootstrapTokenValue("");
    setBootstrapExpiresAtUnixMs(undefined);
  }, [resolvedSelectedPolicyId, open]);

  const bootstrapHelperText = bootstrapError
    ? bootstrapError
    : bootstrapExpiresAtUnixMs
      ? locale === "ru"
        ? `Токен активен до ${new Date(bootstrapExpiresAtUnixMs).toLocaleString(
            locale
          )}.`
        : `Token is valid until ${new Date(bootstrapExpiresAtUnixMs).toLocaleString(
            locale
          )}.`
      : locale === "ru"
        ? "Токен будет выпущен через `POST /api/v1/agents/bootstrap-tokens`."
        : "The token will be issued through `POST /api/v1/agents/bootstrap-tokens`.";

  const handleIssueBootstrapToken = async () => {
    if (!selectedPolicy) {
      return;
    }

    setBootstrapLoading(true);
    setBootstrapError(undefined);

    try {
      const revisionsResponse = await getPolicyRevisions(selectedPolicy.id);
      const revisions = [...revisionsResponse.items];
      const revision =
        revisions.find((item) => item.revision === selectedPolicy.revision) ??
        revisions.sort((left, right) => {
          return (
            new Date(right.created_at).getTime() -
            new Date(left.created_at).getTime()
          );
        })[0];

      if (!revision?.policy_revision_id) {
        throw new Error(
          locale === "ru"
            ? "Edge API не вернул policy_revision_id для выбранной policy."
            : "Edge API did not return a policy_revision_id for the selected policy."
        );
      }

      const response = await issueBootstrapToken({
        policyId: selectedPolicy.id,
        policyRevisionId: revision.policy_revision_id,
        requestedBy:
          agentName.trim() || hostname.trim() || environment.trim() || "web-ui",
        expiresAtUnixMs: Date.now() + 60 * 60 * 1000,
      });

      setBootstrapTokenValue(response.data.bootstrapToken);
      setBootstrapExpiresAtUnixMs(response.data.expiresAtUnixMs);
      showToast({
        title:
          locale === "ru"
            ? "Bootstrap-токен выдан"
            : "Bootstrap token issued",
        description:
          locale === "ru"
            ? "Enrollment-команда обновлена актуальным токеном."
            : "The enrollment command was updated with the new token.",
        variant: "success",
      });
    } catch (error) {
      setBootstrapError(
        error instanceof Error
          ? error.message
          : locale === "ru"
            ? "Не удалось выдать bootstrap-токен."
            : "Failed to issue a bootstrap token."
      );
    } finally {
      setBootstrapLoading(false);
    }
  };

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
                  <Badge variant="success">{liveBadge}</Badge>
                </div>

                <p className="max-w-3xl text-sm leading-6 text-[color:var(--muted-foreground)]">
                  {description}
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
              title={noticeTitle}
              description={noticeDescription}
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
                  value={bootstrapTokenValue || bootstrapTokenPlaceholder}
                  readOnly
                  helperText={bootstrapHelperText}
                />

                <Button
                  variant="outline"
                  size="sm"
                  className="h-10 px-4"
                  loading={bootstrapLoading}
                  disabled={!selectedPolicy}
                  onClick={() => void handleIssueBootstrapToken()}
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
                    {commandDescription}
                  </p>
                </div>

                <pre className="overflow-x-auto rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] p-3 text-xs leading-6 text-[color:var(--foreground)]">
                  <code>{commandPreview}</code>
                </pre>
              </div>
            </div>

            <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[color:var(--border)] pt-4">
              <p className="max-w-3xl text-sm leading-6 text-[color:var(--muted-foreground)]">
                {footerText}
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
                <Button
                  size="sm"
                  className="h-10 px-4"
                  loading={bootstrapLoading}
                  disabled={!selectedPolicy}
                  onClick={() => void handleIssueBootstrapToken()}
                >
                  {copy.bootstrap.button}
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
