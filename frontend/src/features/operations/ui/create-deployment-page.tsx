"use client";

import { startTransition, useEffect, useMemo, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useI18n } from "@/src/shared/lib/i18n";
import { Button, FormLabel, Select, useToast } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import {
  createDeployment,
  listPolicies,
  previewDeploymentPlan,
  type DeploymentMutationPayload,
} from "../api";
import { useApiQuery } from "../model";
import {
  ErrorState,
  JsonPreview,
  NoticeBanner,
  PageStack,
  RequestMetaLine,
  SectionCard,
  TextAreaField,
} from "./operations-ui";

const copyByLocale = {
  en: {
    selectPolicy: "Select a policy",
    planPreviewLoaded: {
      title: "Plan preview loaded",
      description: "The backend returned a deployment plan preview.",
    },
    deploymentCreated: {
      title: "Deployment created",
      description: (id: string) => `Job ${id} was accepted by the backend.`,
    },
    createError: "Failed to create deployment.",
    page: {
      title: "Create Deployment",
      description:
        "Build a deployment request only from fields that are confirmed in the frontend's current public HTTP contract.",
    },
    notice: {
      title: "Partial contract visibility",
      description:
        "This UI only collects confirmed HTTP fields: `policy_id`, optional `agent_ids`, and optional `params`. Target groups, credentials, job type, and other deployment-plane fields are not exposed here until the public HTTP schema is finalized.",
    },
    builder: {
      title: "Request Builder",
      description:
        "Uses `GET /api/v1/policies`, `POST /api/v1/deployments/plan`, and `POST /api/v1/deployments`.",
      preview: "Preview Plan",
      create: "Create Deployment",
      policy: "Policy",
      lookupFailed: "Policies lookup failed",
      agentIds: "Agent IDs (optional)",
      agentIdsHelp:
        "One agent id per line or comma-separated. These are sent only when provided.",
      params: "Params JSON (optional)",
      paramsHelp:
        "Provide a flat JSON object. Values will be serialized to strings, matching the current documented HTTP shape.",
      previewTitle: "Request Preview",
      previewDescription:
        "This is the exact payload assembled from known fields.",
    },
    response: {
      title: "Plan Response",
      description:
        "The preview result is shown exactly as returned by the backend.",
      loading: "Loading plan preview...",
      empty: "Run a plan preview to inspect the backend response.",
    },
    validation: {
      selectPolicy:
        "Select a policy before previewing or creating a deployment.",
      paramsObject: "Params must be a JSON object.",
      paramsJson: "Params must be valid JSON.",
    },
  },
  ru: {
    selectPolicy: "Выберите политику",
    planPreviewLoaded: {
      title: "Превью плана загружено",
      description: "Бэкенд вернул превью deployment-плана.",
    },
    deploymentCreated: {
      title: "Раскатка создана",
      description: (id: string) => `Задача ${id} принята бэкендом.`,
    },
    createError: "Не удалось создать deployment.",
    page: {
      title: "Создание раскатки",
      description:
        "Соберите deployment-запрос только из полей, которые подтверждены текущим публичным HTTP-контрактом фронтенда.",
    },
    notice: {
      title: "Частичная видимость контракта",
      description:
        "Этот UI собирает только подтвержденные HTTP-поля: `policy_id`, опциональные `agent_ids` и опциональные `params`. Target groups, credentials, job type и другие deployment-поля не показываются, пока публичная HTTP-схема не финализирована.",
    },
    builder: {
      title: "Конструктор запроса",
      description:
        "Использует `GET /api/v1/policies`, `POST /api/v1/deployments/plan` и `POST /api/v1/deployments`.",
      preview: "Превью плана",
      create: "Создать раскатку",
      policy: "Политика",
      lookupFailed: "Не удалось получить список политик",
      agentIds: "Agent IDs (опционально)",
      agentIdsHelp:
        "Один agent id на строку или через запятую. Поле отправляется только если заполнено.",
      params: "Params JSON (опционально)",
      paramsHelp:
        "Передайте плоский JSON-объект. Значения будут сериализованы в строки в соответствии с текущей documented HTTP shape.",
      previewTitle: "Превью запроса",
      previewDescription:
        "Это точный payload, собранный из известных полей.",
    },
    response: {
      title: "Ответ плана",
      description:
        "Результат превью показывается ровно в том виде, в каком его вернул бэкенд.",
      loading: "Загрузка превью плана...",
      empty: "Запустите превью плана, чтобы посмотреть ответ бэкенда.",
    },
    validation: {
      selectPolicy:
        "Выберите политику перед превью или созданием раскатки.",
      paramsObject: "Params должны быть JSON-объектом.",
      paramsJson: "Params должны быть валидным JSON.",
    },
  },
} as const;

export function CreateDeploymentPage() {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const { showToast } = useToast();

  const policiesQuery = useApiQuery({
    queryFn: (signal) => listPolicies({ signal }),
    deps: [],
  });

  const requestedPolicyId = searchParams.get("policy_id") ?? "";
  const [policyId, setPolicyId] = useState(requestedPolicyId);
  const [agentIdsText, setAgentIdsText] = useState("");
  const [paramsText, setParamsText] = useState("");
  const [formError, setFormError] = useState<string>();
  const [paramsError, setParamsError] = useState<string>();
  const [planResult, setPlanResult] = useState<unknown>();
  const [planMeta, setPlanMeta] = useState<unknown>();
  const [planError, setPlanError] = useState<unknown>();
  const [planLoading, setPlanLoading] = useState(false);
  const [createLoading, setCreateLoading] = useState(false);

  useEffect(() => {
    if (requestedPolicyId) {
      setPolicyId((current) => current || requestedPolicyId);
    }
  }, [requestedPolicyId]);

  const policyOptions = useMemo(() => {
    const items = policiesQuery.data?.items ?? [];
    return [
      { value: "", label: copy.selectPolicy },
      ...items.map((policy) => ({
        value: policy.id,
        label: `${policy.name} (${policy.id})`,
      })),
    ];
  }, [copy.selectPolicy, policiesQuery.data?.items]);

  const parsedPayload = parseDraftPayload(locale, {
    policyId,
    agentIdsText,
    paramsText,
  });

  const requestPayloadPreview =
    parsedPayload.payload ??
    ({
      policyId: policyId.trim(),
      agentIds:
        agentIdsText
          .split(/\r?\n|,/)
          .map((item) => item.trim())
          .filter(Boolean) || [],
    } as DeploymentMutationPayload);

  const updatePolicyQueryParam = (nextPolicyId: string) => {
    const nextParams = new URLSearchParams(searchParams.toString());
    if (nextPolicyId) {
      nextParams.set("policy_id", nextPolicyId);
    } else {
      nextParams.delete("policy_id");
    }

    startTransition(() => {
      router.replace(
        nextParams.toString() ? `${pathname}?${nextParams}` : pathname,
        { scroll: false }
      );
    });
  };

  const handlePlan = async () => {
    setFormError(undefined);
    setParamsError(undefined);
    setPlanError(undefined);

    if (parsedPayload.error) {
      setFormError(parsedPayload.error);
      if (parsedPayload.field === "params") {
        setParamsError(parsedPayload.error);
      }
      return;
    }

    const payload = parsedPayload.payload;
    if (!payload) {
      return;
    }
    setPlanLoading(true);
    try {
      const result = await previewDeploymentPlan(payload);
      setPlanResult(result.data);
      setPlanMeta(result.meta);
      showToast({
        title: copy.planPreviewLoaded.title,
        description: copy.planPreviewLoaded.description,
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

    if (parsedPayload.error) {
      setFormError(parsedPayload.error);
      if (parsedPayload.field === "params") {
        setParamsError(parsedPayload.error);
      }
      return;
    }

    const payload = parsedPayload.payload;
    if (!payload) {
      return;
    }
    setCreateLoading(true);
    try {
      const result = await createDeployment(payload);
      showToast({
        title: copy.deploymentCreated.title,
        description: copy.deploymentCreated.description(result.data.id),
        variant: "success",
      });

      startTransition(() => {
        router.push(`/${locale}/deployments/${result.data.id}`);
      });
    } catch (caughtError) {
      setFormError(
        caughtError instanceof Error
          ? caughtError.message
          : copy.createError
      );
    } finally {
      setCreateLoading(false);
    }
  };

  return (
    <PageStack>
      <PageHeader
        title={copy.page.title}
        description={copy.page.description}
      />

      <NoticeBanner
        title={copy.notice.title}
        description={copy.notice.description}
      />

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
              onClick={() => void handlePlan()}
              disabled={createLoading}
            >
              {copy.builder.preview}
            </Button>
            <Button
              size="sm"
              className="h-10 px-4"
              loading={createLoading}
              onClick={() => void handleCreate()}
              disabled={planLoading}
            >
              {copy.builder.create}
            </Button>
          </div>
        }
      >
        <div className="space-y-5">
          <div className="space-y-2">
            <FormLabel>{copy.builder.policy}</FormLabel>
            <Select
              value={policyId}
              onChange={(event) => {
                setPolicyId(event.target.value);
                updatePolicyQueryParam(event.target.value);
              }}
              options={policyOptions}
              searchable
              selectSize="lg"
              disabled={policiesQuery.isLoading}
            />
            {policiesQuery.error ? (
              <ErrorState
                title={copy.builder.lookupFailed}
                error={policiesQuery.error}
                retry={() => void policiesQuery.refetch()}
              />
            ) : (
              <RequestMetaLine meta={policiesQuery.meta} />
            )}
          </div>

          <TextAreaField
            id="agent_ids"
            label={copy.builder.agentIds}
            helperText={copy.builder.agentIdsHelp}
            value={agentIdsText}
            onChange={(event) => setAgentIdsText(event.target.value)}
            placeholder={"agent-01\nagent-02"}
          />

          <TextAreaField
            id="params_json"
            label={copy.builder.params}
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

          <SectionCard
            title={copy.builder.previewTitle}
            description={copy.builder.previewDescription}
            className="border border-[color:var(--border)] bg-[color:var(--surface)]"
          >
            <JsonPreview value={serializePayloadPreview(requestPayloadPreview)} />
          </SectionCard>
        </div>
      </SectionCard>

      <SectionCard
        title={copy.response.title}
        description={copy.response.description}
      >
        {planLoading ? (
          <div className="text-sm text-[color:var(--muted-foreground)]">
            {copy.response.loading}
          </div>
        ) : planError ? (
          <ErrorState error={planError as never} />
        ) : planResult ? (
          <div className="space-y-4">
            <RequestMetaLine meta={planMeta as never} />
            <JsonPreview value={planResult} />
          </div>
        ) : (
          <div className="text-sm text-[color:var(--muted-foreground)]">
            {copy.response.empty}
          </div>
        )}
      </SectionCard>
    </PageStack>
  );
}

function parseDraftPayload(
  locale: "ru" | "en",
  input: {
  policyId: string;
  agentIdsText: string;
  paramsText: string;
}):
  | { payload: DeploymentMutationPayload; error?: undefined; field?: undefined }
  | { payload?: undefined; error: string; field?: "policy" | "params" } {
  const copy = copyByLocale[locale];
  const policyId = input.policyId.trim();
  if (!policyId) {
    return {
      error: copy.validation.selectPolicy,
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
      if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        return {
          error: copy.validation.paramsObject,
          field: "params",
        };
      }

      params = Object.fromEntries(
        Object.entries(parsed).map(([key, value]) => [key, String(value)])
      );
    } catch {
      return {
        error: copy.validation.paramsJson,
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

function serializePayloadPreview(payload: DeploymentMutationPayload | undefined) {
  if (!payload) {
    return undefined;
  }

  return {
    policy_id: payload.policyId,
    ...(payload.agentIds && payload.agentIds.length > 0
      ? { agent_ids: payload.agentIds }
      : {}),
    ...(payload.params && Object.keys(payload.params).length > 0
      ? { params: payload.params }
      : {}),
  };
}
