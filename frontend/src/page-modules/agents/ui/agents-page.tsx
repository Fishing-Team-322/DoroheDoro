"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { ApiResponseMeta } from "@/src/shared/lib/api";
import { useApiQuery } from "@/src/features/operations/model";
import {
  DetailGrid,
  ErrorState,
  JsonPreview,
  LoadingState,
  MetricCard,
  NoticeBanner,
  RequestMetaLine,
  SectionCard,
  StatusBadge,
  TextAreaField,
  formatDateTime,
  formatMaybeValue,
  formatNumber,
  formatParamsSummary,
} from "@/src/features/operations/ui/operations-ui";
import {
  translateValueLabel,
  useI18n,
  withLocalePath,
} from "@/src/shared/lib/i18n";
import {
  canCancelDeployment,
  canRetryDeployment,
  cancelDeployment,
  createDeployment,
  createDeploymentPlan,
  getDeploymentById,
  getDeployments,
  getPolicies,
  getPolicyById,
  retryDeployment,
  type DeploymentMutationPayload,
  type DeploymentPlan,
} from "@/src/shared/lib/runtime-api";
import {
  Badge,
  Button,
  Card,
  EmptyState,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  useToast,
} from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { AgentEnrollmentDialog } from "./agent-enrollment-dialog";

const copyByLocale = {
  en: {
    common: {
      na: "n/a",
      loading: "...",
    },
    header: {
      title: "Agents",
      description:
        "Operator-facing agent workspace wired only to confirmed public Edge API endpoints. Policy selection and deployments work today; registry, diagnostics, and enrollment still await Edge bridges.",
    },
    toasts: {
      planLoadedTitle: "Plan preview loaded",
      planLoadedDescription:
        "The deployment plan preview was returned by the public Edge API.",
      deploymentCreatedTitle: "Deployment created",
      deploymentCreatedDescription: (jobId: string) =>
        `Job ${jobId} was accepted by the public Edge API.`,
      retryRequestedTitle: "Retry requested",
      retryRequestedDescription: (deploymentId: string) =>
        `Deployment ${deploymentId} was sent to the retry endpoint.`,
      cancelRequestedTitle: "Cancellation requested",
      cancelRequestedDescription: (deploymentId: string) =>
        `Deployment ${deploymentId} was sent to the cancel endpoint.`,
    },
    notice: {
      title: "Public Edge API scope",
      description:
        "This workspace uses only confirmed HTTP endpoints for policies and deployments. Agent registry, diagnostics, and bootstrap token issuance remain disabled until the missing public Edge bridges are exposed.",
    },
    metrics: {
      policies: {
        label: "Policies",
        hint: "Public `GET /api/v1/policies`",
      },
      activePolicies: {
        label: "Active policies",
        hint: "Policies marked active by Edge",
      },
      inflightJobs: {
        label: "Jobs in flight",
        hint: "Accepted, queued, or running",
      },
      retryableJobs: {
        label: "Retryable jobs",
        hint: "Failed, partial success, or cancelled",
      },
    },
    registry: {
      title: "Agent Registry",
      description:
        "The current public Edge API does not expose `/api/v1/agents` or diagnostics endpoints to WEB, so this table stays honest instead of mirroring internal gRPC or NATS state.",
      createAgent: "Create Agent",
      columns: {
        host: "Host",
        status: "Status",
        policy: "Policy",
        lastSeen: "Last seen",
      },
      emptyTitle: "Agent registry bridge is not available yet",
      emptyDescription:
        "Use the policy and deployment controls below today. Real agent list and diagnostics can be connected once Edge exposes public HTTP endpoints for them.",
    },
    enrollment: {
      title: "Enrollment Bridge",
      description:
        "Frontend structure is ready for future enrollment work, but bootstrap issuance remains disabled until Edge exposes the missing bridge.",
      registryApi: "Registry API",
      diagnosticsApi: "Diagn. API",
      missingBridge: "Missing bridge",
      issueBootstrapToken: "Issue Bootstrap Token",
      openInventory: "Open Inventory",
    },
    policies: {
      title: "Policies",
      description:
        "Confirmed public sources: `GET /api/v1/policies` and `GET /api/v1/policies/{id}`.",
      loading: "Loading policies...",
      columns: {
        name: "Name",
        status: "Status",
        revision: "Revision",
        updated: "Updated",
      },
      emptyTitle: "No policies were returned",
      emptyDescription:
        "Create policies through WEB or the public API before building deployments from this workspace.",
    },
    builder: {
      title: "Deployment Builder",
      description:
        "Preview and create deployments using only confirmed HTTP fields: `policy_id`, optional `agent_ids`, and optional `params`.",
      previewPlan: "Preview Plan",
      createDeployment: "Create Deployment",
      loadingSelectedPolicy: "Loading selected policy...",
      policyDetailsFailed: "Policy details request failed",
      noPolicySelectedTitle: "No policy selected",
      noPolicySelectedDescription:
        "Pick a policy from the table to preview or create deployments.",
      fields: {
        policy: "Policy",
        policyId: "Policy ID",
        status: "Status",
        revision: "Revision",
        updated: "Updated",
        description: "Description",
        executor: "Executor",
        targets: "Targets",
        bootstrapPreviews: "Bootstrap previews",
        warnings: "Warnings",
      },
      policyJsonEmpty:
        "The selected policy response does not include a materialized JSON body.",
      agentIdsLabel: "Explicit agent IDs (optional)",
      agentIdsHelp:
        "Paste agent IDs only if you already know them. There is no public agent registry endpoint to populate this field automatically yet.",
      paramsLabel: "Params JSON (optional)",
      paramsHelp:
        "Provide a flat JSON object. Values are serialized to strings to match the currently confirmed public HTTP shape.",
      planPreviewTitle: "Plan preview",
      planPreviewDescription:
        "The preview stays empty until the public `POST /api/v1/deployments/plan` endpoint is called.",
      loadingPlan: "Loading plan preview...",
      previewTargets: "Preview targets",
      unnamedTarget: "Unnamed target",
      ipUnavailable: "IP unavailable",
      viaUser: (user: string) => ` via ${user}`,
      noPlanTitle: "No plan preview yet",
      noPlanDescription:
        "Select a policy and run Preview Plan to inspect the deployment result available from the public Edge API.",
    },
    jobs: {
      title: "Deployment Jobs",
      description:
        "Confirmed public sources: `GET /api/v1/deployments`, `POST /api/v1/deployments/{id}/retry`, and `POST /api/v1/deployments/{id}/cancel`.",
      loading: "Loading deployment jobs...",
      columns: {
        job: "Job",
        status: "Status",
        policy: "Policy",
        targets: "Targets",
        updated: "Updated",
      },
      emptyTitle: "No deployment jobs were returned",
      emptyDescription:
        "Create a deployment from the builder above or through WEB to populate this list.",
    },
    jobDetails: {
      title: "Job Details",
      description: "Confirmed public source: `GET /api/v1/deployments/{id}`.",
      retry: "Retry",
      cancel: "Cancel",
      openFullDetails: "Open Full Details",
      noJobSelectedTitle: "No job selected",
      noJobSelectedDescription:
        "Choose a deployment job from the table to inspect its public details.",
      loading: "Loading deployment details...",
      fields: {
        deploymentId: "Deployment ID",
        status: "Status",
        policyId: "Policy ID",
        jobType: "Job type",
        currentPhase: "Current phase",
        updated: "Updated",
        attempts: "Attempts",
        targets: "Targets",
        params: "Params",
      },
      attemptsTitle: "Attempts",
      noAttemptsTitle: "No attempts were returned",
      noAttemptsDescription:
        "Attempts will appear after the deployment enters execution.",
      attemptLabel: (attemptNo: string) => `Attempt ${attemptNo}`,
      attemptMeta: (triggeredBy: string, createdAt: string) =>
        `Triggered by ${triggeredBy} at ${createdAt}`,
      targetsTitle: "Targets",
      noTargetsTitle: "No targets were returned",
      noTargetsDescription:
        "Targets will appear once the deployment has been planned or started.",
      latestStepPayloadTitle: "Latest step payload",
      latestStepUpdated: (updatedAt: string) => `Updated ${updatedAt}`,
      noStepsTitle: "No steps were returned",
      noStepsDescription:
        "Step payloads will appear once the job starts executing.",
    },
    validation: {
      policyRequired:
        "Select a policy before previewing or creating a deployment.",
      paramsMustBeObject: "Params must be a JSON object.",
      paramsMustBeJson: "Params must be valid JSON.",
      createFailed: "Failed to create deployment.",
    },
  },
  ru: {
    common: {
      na: "н/д",
      loading: "...",
    },
    header: {
      title: "Агенты",
      description:
        "Операторское рабочее пространство агентов, подключенное только к подтвержденным публичным endpoint'ам Edge API. Выбор политик и раскатки уже работают, а реестр, диагностика и enrollment пока ждут мосты со стороны Edge.",
    },
    toasts: {
      planLoadedTitle: "Предпросмотр плана загружен",
      planLoadedDescription:
        "Публичный Edge API вернул предпросмотр плана раскатки.",
      deploymentCreatedTitle: "Раскатка создана",
      deploymentCreatedDescription: (jobId: string) =>
        `Задача ${jobId} была принята публичным Edge API.`,
      retryRequestedTitle: "Повтор запрошен",
      retryRequestedDescription: (deploymentId: string) =>
        `Раскатка ${deploymentId} была отправлена на endpoint повтора.`,
      cancelRequestedTitle: "Отмена запрошена",
      cancelRequestedDescription: (deploymentId: string) =>
        `Раскатка ${deploymentId} была отправлена на endpoint отмены.`,
    },
    notice: {
      title: "Покрытие публичного Edge API",
      description:
        "Это рабочее пространство использует только подтвержденные HTTP endpoint'ы для политик и раскаток. Реестр агентов, диагностика и выдача bootstrap-токенов остаются отключенными, пока не появятся недостающие публичные мосты Edge.",
    },
    metrics: {
      policies: {
        label: "Политики",
        hint: "Публичный `GET /api/v1/policies`",
      },
      activePolicies: {
        label: "Активные политики",
        hint: "Политики, помеченные Edge как активные",
      },
      inflightJobs: {
        label: "Задачи в работе",
        hint: "Принятые, в очереди или выполняющиеся",
      },
      retryableJobs: {
        label: "Задачи для повтора",
        hint: "Ошибка, частичный успех или отмена",
      },
    },
    registry: {
      title: "Реестр агентов",
      description:
        "Текущий публичный Edge API не отдает `/api/v1/agents` и endpoint'ы диагностики в WEB, поэтому эта таблица честно остается пустой и не притворяется зеркалом внутреннего gRPC или NATS-состояния.",
      createAgent: "Создать агента",
      columns: {
        host: "Хост",
        status: "Статус",
        policy: "Политика",
        lastSeen: "Последняя активность",
      },
      emptyTitle: "Мост к реестру агентов пока недоступен",
      emptyDescription:
        "Пока можно использовать блоки политик и раскаток ниже. Реальный список агентов и диагностика появятся, когда Edge откроет для них публичные HTTP endpoint'ы.",
    },
    enrollment: {
      title: "Мост enrollment",
      description:
        "Структура фронтенда уже готова к будущему enrollment-сценарию, но выдача bootstrap пока отключена, пока Edge не откроет недостающий мост.",
      registryApi: "API реестра",
      diagnosticsApi: "API диагностик",
      missingBridge: "Недостающий мост",
      issueBootstrapToken: "Выдать bootstrap-токен",
      openInventory: "Открыть inventory",
    },
    policies: {
      title: "Политики",
      description:
        "Подтвержденные публичные источники: `GET /api/v1/policies` и `GET /api/v1/policies/{id}`.",
      loading: "Загружаем политики...",
      columns: {
        name: "Название",
        status: "Статус",
        revision: "Ревизия",
        updated: "Обновлено",
      },
      emptyTitle: "Политики не были возвращены",
      emptyDescription:
        "Создайте политики через WEB или публичный API, прежде чем собирать раскатки из этого рабочего пространства.",
    },
    builder: {
      title: "Конструктор раскаток",
      description:
        "Предпросмотр и создание раскаток только по подтвержденным HTTP-полям: `policy_id`, опциональным `agent_ids` и опциональным `params`.",
      previewPlan: "Предпросмотр плана",
      createDeployment: "Создать раскатку",
      loadingSelectedPolicy: "Загружаем выбранную политику...",
      policyDetailsFailed: "Не удалось загрузить детали политики",
      noPolicySelectedTitle: "Политика не выбрана",
      noPolicySelectedDescription:
        "Выберите политику в таблице, чтобы посмотреть предпросмотр или создать раскатку.",
      fields: {
        policy: "Политика",
        policyId: "ID политики",
        status: "Статус",
        revision: "Ревизия",
        updated: "Обновлено",
        description: "Описание",
        executor: "Исполнитель",
        targets: "Цели",
        bootstrapPreviews: "Bootstrap-превью",
        warnings: "Предупреждения",
      },
      policyJsonEmpty:
        "Ответ по выбранной политике не содержит materialized JSON body.",
      agentIdsLabel: "Явные ID агентов (опционально)",
      agentIdsHelp:
        "Вставляйте ID агентов, только если уже знаете их заранее. Публичного endpoint'а реестра агентов, чтобы заполнить это поле автоматически, пока нет.",
      paramsLabel: "Params JSON (опционально)",
      paramsHelp:
        "Передайте плоский JSON-объект. Значения будут сериализованы в строки, чтобы соответствовать текущей подтвержденной публичной HTTP-схеме.",
      planPreviewTitle: "Предпросмотр плана",
      planPreviewDescription:
        "Этот блок остается пустым, пока не будет вызван публичный endpoint `POST /api/v1/deployments/plan`.",
      loadingPlan: "Загружаем предпросмотр плана...",
      previewTargets: "Цели предпросмотра",
      unnamedTarget: "Цель без имени",
      ipUnavailable: "IP недоступен",
      viaUser: (user: string) => ` через ${user}`,
      noPlanTitle: "Предпросмотра плана пока нет",
      noPlanDescription:
        "Выберите политику и запустите предпросмотр плана, чтобы изучить результат раскатки из публичного Edge API.",
    },
    jobs: {
      title: "Задачи раскаток",
      description:
        "Подтвержденные публичные источники: `GET /api/v1/deployments`, `POST /api/v1/deployments/{id}/retry` и `POST /api/v1/deployments/{id}/cancel`.",
      loading: "Загружаем задачи раскаток...",
      columns: {
        job: "Задача",
        status: "Статус",
        policy: "Политика",
        targets: "Цели",
        updated: "Обновлено",
      },
      emptyTitle: "Задачи раскаток не были возвращены",
      emptyDescription:
        "Создайте раскатку в конструкторе выше или через WEB, чтобы заполнить этот список.",
    },
    jobDetails: {
      title: "Детали задачи",
      description:
        "Подтвержденный публичный источник: `GET /api/v1/deployments/{id}`.",
      retry: "Повторить",
      cancel: "Отменить",
      openFullDetails: "Открыть полные детали",
      noJobSelectedTitle: "Задача не выбрана",
      noJobSelectedDescription:
        "Выберите задачу раскатки в таблице, чтобы посмотреть ее публичные детали.",
      loading: "Загружаем детали раскатки...",
      fields: {
        deploymentId: "ID раскатки",
        status: "Статус",
        policyId: "ID политики",
        jobType: "Тип задачи",
        currentPhase: "Текущая фаза",
        updated: "Обновлено",
        attempts: "Попытки",
        targets: "Цели",
        params: "Параметры",
      },
      attemptsTitle: "Попытки",
      noAttemptsTitle: "Попытки не были возвращены",
      noAttemptsDescription:
        "Попытки появятся после того, как раскатка перейдет к выполнению.",
      attemptLabel: (attemptNo: string) => `Попытка ${attemptNo}`,
      attemptMeta: (triggeredBy: string, createdAt: string) =>
        `Запущено пользователем ${triggeredBy} в ${createdAt}`,
      targetsTitle: "Цели",
      noTargetsTitle: "Цели не были возвращены",
      noTargetsDescription:
        "Цели появятся после того, как раскатка будет спланирована или запущена.",
      latestStepPayloadTitle: "Payload последнего шага",
      latestStepUpdated: (updatedAt: string) => `Обновлено ${updatedAt}`,
      noStepsTitle: "Шаги не были возвращены",
      noStepsDescription:
        "Payload шагов появятся после того, как задача начнет выполняться.",
    },
    validation: {
      policyRequired:
        "Сначала выберите политику, прежде чем смотреть предпросмотр или создавать раскатку.",
      paramsMustBeObject: "Параметры должны быть JSON-объектом.",
      paramsMustBeJson: "Параметры должны быть валидным JSON.",
      createFailed: "Не удалось создать раскатку.",
    },
  },
} as const;

function formatTranslatedValue(
  value: string | null | undefined,
  locale: "ru" | "en"
) {
  if (!value) {
    return formatMaybeValue(value, locale);
  }

  return translateValueLabel(value, locale);
}

export function AgentsPage({ embedded = false }: { embedded?: boolean } = {}) {
  const { dictionary, locale } = useI18n();
  const copy = copyByLocale[locale];
  const { showToast } = useToast();

  const [createAgentDialogOpen, setCreateAgentDialogOpen] = useState(false);
  const [selectedPolicyId, setSelectedPolicyId] = useState<string | null>(null);
  const [selectedDeploymentId, setSelectedDeploymentId] = useState<
    string | null
  >(null);
  const [agentIdsText, setAgentIdsText] = useState("");
  const [paramsText, setParamsText] = useState("");
  const [formError, setFormError] = useState<string>();
  const [paramsError, setParamsError] = useState<string>();
  const [planResult, setPlanResult] = useState<DeploymentPlan>();
  const [planMeta, setPlanMeta] = useState<ApiResponseMeta>();
  const [planError, setPlanError] = useState<unknown>();
  const [planLoading, setPlanLoading] = useState(false);
  const [createLoading, setCreateLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState<"retry" | "cancel" | null>(
    null
  );
  const [actionError, setActionError] = useState<unknown>();

  const policiesQuery = useApiQuery({
    queryFn: (signal) => getPolicies({ signal }),
    deps: [],
  });

  const policyDetailsQuery = useApiQuery({
    enabled: Boolean(selectedPolicyId),
    queryFn: (signal) => getPolicyById(selectedPolicyId ?? "", signal),
    deps: [selectedPolicyId],
  });

  const deploymentsQuery = useApiQuery({
    queryFn: (signal) => getDeployments({ signal }),
    deps: [],
    pollIntervalMs: 10_000,
  });

  const deploymentDetailsQuery = useApiQuery({
    enabled: Boolean(selectedDeploymentId),
    queryFn: (signal) => getDeploymentById(selectedDeploymentId ?? "", signal),
    deps: [selectedDeploymentId],
    pollIntervalMs: 5_000,
  });

  useEffect(() => {
    const items = policiesQuery.data?.items ?? [];

    if (items.length === 0) {
      setSelectedPolicyId(null);
      return;
    }

    setSelectedPolicyId((current) => {
      if (current && items.some((item) => item.id === current)) {
        return current;
      }

      return items[0]?.id ?? null;
    });
  }, [policiesQuery.data?.items]);

  useEffect(() => {
    const items = deploymentsQuery.data?.items ?? [];

    if (items.length === 0 && !createLoading) {
      setSelectedDeploymentId(null);
      return;
    }

    setSelectedDeploymentId((current) => {
      if (current && items.some((item) => item.id === current)) {
        return current;
      }

      return current ?? items[0]?.id ?? null;
    });
  }, [createLoading, deploymentsQuery.data?.items]);

  useEffect(() => {
    setFormError(undefined);
    setParamsError(undefined);
    setPlanError(undefined);
    setPlanResult(undefined);
    setPlanMeta(undefined);
  }, [selectedPolicyId, agentIdsText, paramsText]);

  const selectedDeployment = deploymentDetailsQuery.data?.summary;
  const activePoliciesCount =
    policiesQuery.data?.items.filter((item) => item.isActive === true).length ??
    0;
  const inflightDeploymentsCount =
    deploymentsQuery.data?.items.filter((item) =>
      canCancelDeployment(item.status)
    ).length ?? 0;
  const retryableDeploymentsCount =
    deploymentsQuery.data?.items.filter((item) =>
      canRetryDeployment(item.status)
    ).length ?? 0;

  const handlePlan = async () => {
    setFormError(undefined);
    setParamsError(undefined);
    setPlanError(undefined);

    const parsedPayload = parseDraftPayload({
      policyId: selectedPolicyId ?? "",
      agentIdsText,
      paramsText,
      locale,
    });

    if (parsedPayload.error) {
      setFormError(parsedPayload.error);
      if (parsedPayload.field === "params") {
        setParamsError(parsedPayload.error);
      }
      return;
    }

    if (!parsedPayload.payload) {
      return;
    }

    setPlanLoading(true);
    try {
      const response = await createDeploymentPlan(parsedPayload.payload);
      setPlanResult(response.data);
      setPlanMeta(response.meta);
      showToast({
        title: copy.toasts.planLoadedTitle,
        description: copy.toasts.planLoadedDescription,
      });
    } catch (caughtError) {
      setPlanError(caughtError);
    } finally {
      setPlanLoading(false);
    }
  };

  const handleCreate = async () => {
    setFormError(undefined);
    setParamsError(undefined);

    const parsedPayload = parseDraftPayload({
      policyId: selectedPolicyId ?? "",
      agentIdsText,
      paramsText,
      locale,
    });

    if (parsedPayload.error) {
      setFormError(parsedPayload.error);
      if (parsedPayload.field === "params") {
        setParamsError(parsedPayload.error);
      }
      return;
    }

    if (!parsedPayload.payload) {
      return;
    }

    setCreateLoading(true);
    try {
      const response = await createDeployment(parsedPayload.payload);
      setSelectedDeploymentId(response.data.id);
      showToast({
        title: copy.toasts.deploymentCreatedTitle,
        description: copy.toasts.deploymentCreatedDescription(response.data.id),
        variant: "success",
      });
      await deploymentsQuery.refetch({ silent: true });
    } catch (caughtError) {
      setFormError(
        caughtError instanceof Error
          ? caughtError.message
          : copy.validation.createFailed
      );
    } finally {
      setCreateLoading(false);
    }
  };

  const handleDeploymentAction = async (action: "retry" | "cancel") => {
    if (!selectedDeployment) {
      return;
    }

    setActionError(undefined);
    setActionLoading(action);

    try {
      if (action === "retry") {
        await retryDeployment(selectedDeployment.id);
        showToast({
          title: copy.toasts.retryRequestedTitle,
          description: copy.toasts.retryRequestedDescription(selectedDeployment.id),
          variant: "success",
        });
      } else {
        await cancelDeployment(selectedDeployment.id);
        showToast({
          title: copy.toasts.cancelRequestedTitle,
          description: copy.toasts.cancelRequestedDescription(
            selectedDeployment.id
          ),
          variant: "success",
        });
      }

      await Promise.all([
        deploymentsQuery.refetch({ silent: true }),
        deploymentDetailsQuery.refetch({ silent: true }),
      ]);
    } catch (caughtError) {
      setActionError(caughtError);
    } finally {
      setActionLoading(null);
    }
  };

  const content = (
    <div className="space-y-6">
      <NoticeBanner
        title={copy.notice.title}
        description={copy.notice.description}
      />

      <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label={copy.metrics.policies.label}
          value={
            policiesQuery.data
              ? formatNumber(policiesQuery.data.items.length, locale)
              : policiesQuery.error
                ? copy.common.na
                : copy.common.loading
          }
          hint={copy.metrics.policies.hint}
        />
        <MetricCard
          label={copy.metrics.activePolicies.label}
          value={
            policiesQuery.data
              ? formatNumber(activePoliciesCount, locale)
              : copy.common.loading
          }
          hint={copy.metrics.activePolicies.hint}
        />
        <MetricCard
          label={copy.metrics.inflightJobs.label}
          value={
            deploymentsQuery.data
              ? formatNumber(inflightDeploymentsCount, locale)
              : deploymentsQuery.error
                ? copy.common.na
                : copy.common.loading
          }
          hint={copy.metrics.inflightJobs.hint}
        />
        <MetricCard
          label={copy.metrics.retryableJobs.label}
          value={
            deploymentsQuery.data
              ? formatNumber(retryableDeploymentsCount, locale)
              : deploymentsQuery.error
                ? copy.common.na
                : copy.common.loading
          }
          hint={copy.metrics.retryableJobs.hint}
        />
      </section>

      <section className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,0.8fr)]">
        <SectionCard
          title={copy.registry.title}
          description={copy.registry.description}
          action={
            <Button
              size="sm"
              className="h-10 px-4"
              onClick={() => setCreateAgentDialogOpen(true)}
            >
              {copy.registry.createAgent}
            </Button>
          }
        >
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{copy.registry.columns.host}</TableHead>
                <TableHead>{copy.registry.columns.status}</TableHead>
                <TableHead>{copy.registry.columns.policy}</TableHead>
                <TableHead>{copy.registry.columns.lastSeen}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow>
                <TableCell colSpan={4}>
                  <EmptyState
                    variant="flush"
                    title={copy.registry.emptyTitle}
                    description={copy.registry.emptyDescription}
                  />
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </SectionCard>

        <SectionCard
          title={copy.enrollment.title}
          description={copy.enrollment.description}
        >
          <div className="space-y-4">
            <DetailGrid
              items={[
                {
                  label: copy.enrollment.registryApi,
                  value: <StatusBadge value="unavailable" />,
                },
                {
                  label: copy.enrollment.diagnosticsApi,
                  value: <StatusBadge value="unavailable" />,
                },
                {
                  label: copy.enrollment.missingBridge,
                  value: "agents.bootstrap-token.issue",
                },
              ]}
            />

            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                disabled
              >
                {copy.enrollment.issueBootstrapToken}
              </Button>
              <Link
                href={withLocalePath(locale, "/infrastructure?tab=resources")}
              >
                <Button variant="outline" size="sm" className="h-10 px-4">
                  {copy.enrollment.openInventory}
                </Button>
              </Link>
            </div>
          </div>
        </SectionCard>
      </section>

      <section className="space-y-6">
        <SectionCard
          title={copy.policies.title}
          description={copy.policies.description}
        >
          {policiesQuery.isLoading && !policiesQuery.data ? (
            <LoadingState label={copy.policies.loading} />
          ) : policiesQuery.error && !policiesQuery.data ? (
            <ErrorState
              error={policiesQuery.error}
              retry={() => void policiesQuery.refetch()}
            />
          ) : (
            <div className="space-y-4">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{copy.policies.columns.name}</TableHead>
                    <TableHead>{copy.policies.columns.status}</TableHead>
                    <TableHead>{copy.policies.columns.revision}</TableHead>
                    <TableHead>{copy.policies.columns.updated}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(policiesQuery.data?.items.length ?? 0) === 0 ? (
                    <TableRow>
                      <TableCell colSpan={4}>
                        <EmptyState
                          variant="flush"
                          title={copy.policies.emptyTitle}
                          description={copy.policies.emptyDescription}
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    policiesQuery.data?.items.map((policy) => (
                      <TableRow
                        key={policy.id}
                        className={
                          policy.id === selectedPolicyId
                            ? "bg-transparent"
                            : undefined
                        }
                        onClick={() => setSelectedPolicyId(policy.id)}
                      >
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          {policy.name}
                        </TableCell>
                        <TableCell>
                          <StatusBadge
                            value={
                              policy.isActive === false ? "inactive" : "active"
                            }
                          />
                        </TableCell>
                        <TableCell>
                          {formatMaybeValue(policy.revision, locale)}
                        </TableCell>
                        <TableCell>
                          {formatDateTime(policy.updatedAt, locale)}
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
              <RequestMetaLine meta={policiesQuery.meta} />
            </div>
          )}
        </SectionCard>

        <SectionCard
          title={copy.builder.title}
          description={copy.builder.description}
          action={
            <div className="flex flex-wrap gap-2">
              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                loading={planLoading}
                disabled={!selectedPolicyId || createLoading}
                onClick={() => void handlePlan()}
              >
                {copy.builder.previewPlan}
              </Button>
              <Button
                size="sm"
                className="h-10 px-4"
                loading={createLoading}
                disabled={!selectedPolicyId || planLoading}
                onClick={() => void handleCreate()}
              >
                {copy.builder.createDeployment}
              </Button>
            </div>
          }
        >
          <div className="space-y-5">
            {selectedPolicyId ? (
              policyDetailsQuery.isLoading && !policyDetailsQuery.data ? (
                <LoadingState compact label={copy.builder.loadingSelectedPolicy} />
              ) : policyDetailsQuery.error && !policyDetailsQuery.data ? (
                <ErrorState
                  title={copy.builder.policyDetailsFailed}
                  error={policyDetailsQuery.error}
                  retry={() => void policyDetailsQuery.refetch()}
                />
              ) : policyDetailsQuery.data ? (
                <div className="space-y-4">
                  <DetailGrid
                    items={[
                      {
                        label: copy.builder.fields.policy,
                        value: policyDetailsQuery.data.name,
                      },
                      {
                        label: copy.builder.fields.policyId,
                        value: policyDetailsQuery.data.id,
                      },
                      {
                        label: copy.builder.fields.status,
                        value: (
                          <StatusBadge
                            value={
                              policyDetailsQuery.data.isActive === false
                                ? "inactive"
                                : "active"
                            }
                          />
                        ),
                      },
                      {
                        label: copy.builder.fields.revision,
                        value: formatMaybeValue(
                          policyDetailsQuery.data.revision,
                          locale
                        ),
                      },
                      {
                        label: copy.builder.fields.updated,
                        value: formatDateTime(
                          policyDetailsQuery.data.updatedAt,
                          locale
                        ),
                      },
                      {
                        label: copy.builder.fields.description,
                        value: formatMaybeValue(
                          policyDetailsQuery.data.description,
                          locale
                        ),
                      },
                    ]}
                  />
                  <JsonPreview
                    value={policyDetailsQuery.data.body}
                    emptyLabel={copy.builder.policyJsonEmpty}
                  />
                  <RequestMetaLine meta={policyDetailsQuery.meta} />
                </div>
              ) : null
            ) : (
              <EmptyState
                variant="flush"
                title={copy.builder.noPolicySelectedTitle}
                description={copy.builder.noPolicySelectedDescription}
              />
            )}

            <TextAreaField
              id="agent_ids"
              label={copy.builder.agentIdsLabel}
              helperText={copy.builder.agentIdsHelp}
              value={agentIdsText}
              onChange={(event) => setAgentIdsText(event.target.value)}
              placeholder={"agent-01\nagent-02"}
            />

            <TextAreaField
              id="params_json"
              label={copy.builder.paramsLabel}
              helperText={copy.builder.paramsHelp}
              value={paramsText}
              onChange={(event) => setParamsText(event.target.value)}
              placeholder='{"rollout":"canary","window":"15m"}'
              error={paramsError}
            />

            {formError ? (
              <p className="text-sm text-[color:var(--status-danger-fg)]">
                {formError}
              </p>
            ) : null}

            <div className="rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <div className="space-y-4">
                <div>
                  <p className="text-sm font-semibold text-[color:var(--foreground)]">
                    {copy.builder.planPreviewTitle}
                  </p>
                  <p className="mt-1 text-sm leading-6 text-[color:var(--muted-foreground)]">
                    {copy.builder.planPreviewDescription}
                  </p>
                </div>

                {planLoading ? (
                  <LoadingState compact label={copy.builder.loadingPlan} />
                ) : planError ? (
                  <ErrorState error={planError as never} />
                ) : planResult ? (
                  <div className="space-y-4">
                    <DetailGrid
                      items={[
                        {
                          label: copy.builder.fields.policyId,
                          value: formatMaybeValue(planResult.policyId, locale),
                        },
                        {
                          label: copy.builder.fields.revision,
                          value: formatMaybeValue(
                            planResult.policyRevision,
                            locale
                          ),
                        },
                        {
                          label: copy.builder.fields.executor,
                          value: formatTranslatedValue(
                            planResult.executorKind,
                            locale
                          ),
                        },
                        {
                          label: copy.builder.fields.targets,
                          value: formatNumber(planResult.targets.length, locale),
                        },
                        {
                          label: copy.builder.fields.bootstrapPreviews,
                          value: formatNumber(
                            planResult.bootstrapPreviews.length,
                            locale
                          ),
                        },
                        {
                          label: copy.builder.fields.warnings,
                          value: formatNumber(planResult.warnings.length, locale),
                        },
                      ]}
                    />

                    {planResult.actionSummary ? (
                      <p className="text-sm leading-6 text-[color:var(--foreground)]">
                        {planResult.actionSummary}
                      </p>
                    ) : null}

                    {planResult.warnings.length > 0 ? (
                      <div className="flex flex-wrap gap-2">
                        {planResult.warnings.map((warning) => (
                          <Badge key={warning} variant="warning">
                            {warning}
                          </Badge>
                        ))}
                      </div>
                    ) : null}

                    {planResult.targets.length > 0 ? (
                      <div className="space-y-2">
                        <p className="text-sm font-semibold text-[color:var(--foreground)]">
                          {copy.builder.previewTargets}
                        </p>
                        {planResult.targets.slice(0, 6).map((target, index) => (
                          <div
                            key={`${target.hostId ?? target.hostname ?? "target"}-${index}`}
                            className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-elevated)] p-3 text-sm"
                          >
                            <p className="font-medium text-[color:var(--foreground)]">
                              {target.hostname ??
                                target.hostId ??
                                copy.builder.unnamedTarget}
                            </p>
                            <p className="mt-1 text-[color:var(--muted-foreground)]">
                              {target.ip ?? copy.builder.ipUnavailable}
                              {target.remoteUser
                                ? copy.builder.viaUser(target.remoteUser)
                                : ""}
                            </p>
                          </div>
                        ))}
                      </div>
                    ) : null}

                    <RequestMetaLine meta={planMeta} />
                  </div>
                ) : (
                  <EmptyState
                    variant="flush"
                    title={copy.builder.noPlanTitle}
                    description={copy.builder.noPlanDescription}
                  />
                )}
              </div>
            </div>
          </div>
        </SectionCard>
      </section>

      <section className="grid gap-4 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)]">
        <SectionCard
          title={copy.jobs.title}
          description={copy.jobs.description}
        >
          {deploymentsQuery.isLoading && !deploymentsQuery.data ? (
            <LoadingState label={copy.jobs.loading} />
          ) : deploymentsQuery.error && !deploymentsQuery.data ? (
            <ErrorState
              error={deploymentsQuery.error}
              retry={() => void deploymentsQuery.refetch()}
            />
          ) : (
            <div className="space-y-4">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{copy.jobs.columns.job}</TableHead>
                    <TableHead>{copy.jobs.columns.status}</TableHead>
                    <TableHead>{copy.jobs.columns.policy}</TableHead>
                    <TableHead>{copy.jobs.columns.targets}</TableHead>
                    <TableHead>{copy.jobs.columns.updated}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(deploymentsQuery.data?.items.length ?? 0) === 0 ? (
                    <TableRow>
                      <TableCell colSpan={5}>
                        <EmptyState
                          variant="flush"
                          title={copy.jobs.emptyTitle}
                          description={copy.jobs.emptyDescription}
                        />
                      </TableCell>
                    </TableRow>
                  ) : (
                    deploymentsQuery.data?.items.map((job) => (
                      <TableRow
                        key={job.id}
                        className={
                          job.id === selectedDeploymentId
                            ? "bg-[color:rgba(56,189,248,0.08)]"
                            : undefined
                        }
                        onClick={() => setSelectedDeploymentId(job.id)}
                      >
                        <TableCell className="font-medium text-[color:var(--foreground)]">
                          <div>{formatTranslatedValue(job.jobType, locale)}</div>
                          <div className="mt-1 text-xs text-[color:var(--muted-foreground)]">
                            {job.id}
                          </div>
                        </TableCell>
                        <TableCell>
                          <StatusBadge value={job.status} />
                        </TableCell>
                        <TableCell>{formatMaybeValue(job.policyId, locale)}</TableCell>
                        <TableCell>{formatNumber(job.totalTargets, locale)}</TableCell>
                        <TableCell>{formatDateTime(job.updatedAt, locale)}</TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
              <RequestMetaLine meta={deploymentsQuery.meta} />
            </div>
          )}
        </SectionCard>

        <SectionCard
          title={copy.jobDetails.title}
          description={copy.jobDetails.description}
          action={
            selectedDeployment ? (
              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="h-10 px-4"
                  disabled={
                    !canRetryDeployment(selectedDeployment.status) ||
                    actionLoading !== null
                  }
                  loading={actionLoading === "retry"}
                  onClick={() => void handleDeploymentAction("retry")}
                >
                  {copy.jobDetails.retry}
                </Button>
                <Button
                  variant="danger"
                  size="sm"
                  className="h-10 px-4"
                  disabled={
                    !canCancelDeployment(selectedDeployment.status) ||
                    actionLoading !== null
                  }
                  loading={actionLoading === "cancel"}
                  onClick={() => void handleDeploymentAction("cancel")}
                >
                  {copy.jobDetails.cancel}
                </Button>
                <Link
                  href={withLocalePath(
                    locale,
                    `/deployments/${selectedDeployment.id}`
                  )}
                >
                  <Button variant="outline" size="sm" className="h-10 px-4">
                    {copy.jobDetails.openFullDetails}
                  </Button>
                </Link>
              </div>
            ) : null
          }
        >
          {!selectedDeploymentId ? (
            <EmptyState
              variant="flush"
              title={copy.jobDetails.noJobSelectedTitle}
              description={copy.jobDetails.noJobSelectedDescription}
            />
          ) : deploymentDetailsQuery.isLoading &&
            !deploymentDetailsQuery.data ? (
            <LoadingState label={copy.jobDetails.loading} />
          ) : deploymentDetailsQuery.error && !deploymentDetailsQuery.data ? (
            <ErrorState
              error={deploymentDetailsQuery.error}
              retry={() => void deploymentDetailsQuery.refetch()}
            />
          ) : deploymentDetailsQuery.data ? (
            <div className="space-y-5">
              {actionError ? <ErrorState error={actionError as never} /> : null}

              <DetailGrid
                items={[
                  {
                    label: copy.jobDetails.fields.deploymentId,
                    value: deploymentDetailsQuery.data.summary.id,
                  },
                  {
                    label: copy.jobDetails.fields.status,
                    value: (
                      <StatusBadge
                        value={deploymentDetailsQuery.data.summary.status}
                      />
                    ),
                  },
                  {
                    label: copy.jobDetails.fields.policyId,
                    value: formatMaybeValue(
                      deploymentDetailsQuery.data.summary.policyId,
                      locale
                    ),
                  },
                  {
                    label: copy.jobDetails.fields.jobType,
                    value: formatTranslatedValue(
                      deploymentDetailsQuery.data.summary.jobType,
                      locale
                    ),
                  },
                  {
                    label: copy.jobDetails.fields.currentPhase,
                    value: formatTranslatedValue(
                      deploymentDetailsQuery.data.summary.currentPhase,
                      locale
                    ),
                  },
                  {
                    label: copy.jobDetails.fields.updated,
                    value: formatDateTime(
                      deploymentDetailsQuery.data.summary.updatedAt,
                      locale
                    ),
                  },
                  {
                    label: copy.jobDetails.fields.attempts,
                    value: formatNumber(
                      deploymentDetailsQuery.data.summary.attemptCount,
                      locale
                    ),
                  },
                  {
                    label: copy.jobDetails.fields.targets,
                    value: formatNumber(
                      deploymentDetailsQuery.data.summary.totalTargets,
                      locale
                    ),
                  },
                  {
                    label: copy.jobDetails.fields.params,
                    value: formatParamsSummary(
                      deploymentDetailsQuery.data.summary.params,
                      locale
                    ),
                  },
                ]}
              />

              <div className="space-y-2">
                <p className="text-sm font-semibold text-[color:var(--foreground)]">
                  {copy.jobDetails.attemptsTitle}
                </p>
                {deploymentDetailsQuery.data.attempts.length === 0 ? (
                  <EmptyState
                    variant="flush"
                    title={copy.jobDetails.noAttemptsTitle}
                    description={copy.jobDetails.noAttemptsDescription}
                  />
                ) : (
                  deploymentDetailsQuery.data.attempts.map((attempt) => (
                    <div
                      key={attempt.id}
                      className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3 text-sm"
                    >
                      <div className="flex flex-wrap items-center justify-between gap-3">
                        <span className="font-medium text-[color:var(--foreground)]">
                          {copy.jobDetails.attemptLabel(
                            formatMaybeValue(attempt.attemptNo, locale)
                          )}
                        </span>
                        <StatusBadge value={attempt.status} />
                      </div>
                      <p className="mt-2 text-[color:var(--muted-foreground)]">
                        {copy.jobDetails.attemptMeta(
                          formatTranslatedValue(attempt.triggeredBy, locale),
                          formatDateTime(attempt.createdAt, locale)
                        )}
                      </p>
                    </div>
                  ))
                )}
              </div>

              <div className="space-y-2">
                <p className="text-sm font-semibold text-[color:var(--foreground)]">
                  {copy.jobDetails.targetsTitle}
                </p>
                {deploymentDetailsQuery.data.targets.length === 0 ? (
                  <EmptyState
                    variant="flush"
                    title={copy.jobDetails.noTargetsTitle}
                    description={copy.jobDetails.noTargetsDescription}
                  />
                ) : (
                  deploymentDetailsQuery.data.targets
                    .slice(0, 8)
                    .map((target) => (
                      <div
                        key={target.id}
                        className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3 text-sm"
                      >
                        <div className="flex flex-wrap items-center justify-between gap-3">
                          <span className="font-medium text-[color:var(--foreground)]">
                            {target.hostname ?? target.hostId ?? target.id}
                          </span>
                          <StatusBadge value={target.status} />
                        </div>
                        {target.errorMessage ? (
                          <p className="mt-2 text-[color:var(--status-danger-fg)]">
                            {target.errorMessage}
                          </p>
                        ) : null}
                      </div>
                    ))
                )}
              </div>

              <div className="space-y-2">
                <p className="text-sm font-semibold text-[color:var(--foreground)]">
                  {copy.jobDetails.latestStepPayloadTitle}
                </p>
                {deploymentDetailsQuery.data.steps[0] ? (
                  <div className="space-y-3">
                    <div className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3 text-sm">
                      <div className="flex flex-wrap items-center justify-between gap-3">
                        <span className="font-medium text-[color:var(--foreground)]">
                          {formatMaybeValue(
                            deploymentDetailsQuery.data.steps[0].name,
                            locale
                          )}
                        </span>
                        <StatusBadge
                          value={deploymentDetailsQuery.data.steps[0].status}
                        />
                      </div>
                      <p className="mt-2 text-[color:var(--muted-foreground)]">
                        {copy.jobDetails.latestStepUpdated(
                          formatDateTime(
                            deploymentDetailsQuery.data.steps[0].updatedAt,
                            locale
                          )
                        )}
                      </p>
                    </div>
                    <JsonPreview
                      value={deploymentDetailsQuery.data.steps[0].payload}
                    />
                  </div>
                ) : (
                  <EmptyState
                    variant="flush"
                    title={copy.jobDetails.noStepsTitle}
                    description={copy.jobDetails.noStepsDescription}
                  />
                )}
              </div>

              <RequestMetaLine meta={deploymentDetailsQuery.meta} />
            </div>
          ) : null}
        </SectionCard>
      </section>
    </div>
  );

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title={copy.header.title}
          description={copy.header.description}
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: copy.header.title },
          ]}
        />
      ) : null}

      {embedded ? <div>{content}</div> : <Card className="p-6">{content}</Card>}

      {createAgentDialogOpen ? (
        <AgentEnrollmentDialog
          open={createAgentDialogOpen}
          onClose={() => setCreateAgentDialogOpen(false)}
          policies={policiesQuery.data?.items ?? []}
          initialPolicyId={selectedPolicyId}
          locale={locale}
        />
      ) : null}
    </div>
  );
}

function parseDraftPayload(input: {
  policyId: string;
  agentIdsText: string;
  paramsText: string;
  locale: "ru" | "en";
}):
  | { payload: DeploymentMutationPayload; error?: undefined; field?: undefined }
  | { payload?: undefined; error: string; field?: "policy" | "params" } {
  const copy = copyByLocale[input.locale];
  const policyId = input.policyId.trim();

  if (!policyId) {
    return {
      error: copy.validation.policyRequired,
      field: "policy",
    };
  }

  const agentIds = input.agentIdsText
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean);

  let params: Record<string, string> | undefined;

  if (input.paramsText.trim()) {
    try {
      const parsed = JSON.parse(input.paramsText);

      if (
        typeof parsed !== "object" ||
        parsed === null ||
        Array.isArray(parsed)
      ) {
        return {
          error: copy.validation.paramsMustBeObject,
          field: "params",
        };
      }

      params = Object.fromEntries(
        Object.entries(parsed).map(([key, value]) => [key, String(value)])
      );
    } catch {
      return {
        error: copy.validation.paramsMustBeJson,
        field: "params",
      };
    }
  }

  return {
    payload: {
      policyId,
      ...(agentIds.length > 0 ? { agentIds } : {}),
      ...(params && Object.keys(params).length > 0 ? { params } : {}),
    },
  };
}
