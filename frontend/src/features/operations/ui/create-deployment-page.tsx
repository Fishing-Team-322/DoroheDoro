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

export function CreateDeploymentPage() {
  const { locale } = useI18n();
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
      { value: "", label: "Select a policy" },
      ...items.map((policy) => ({
        value: policy.id,
        label: `${policy.name} (${policy.id})`,
      })),
    ];
  }, [policiesQuery.data?.items]);

  const parsedPayload = parseDraftPayload({
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
        title: "Plan preview loaded",
        description: "The backend returned a deployment plan preview.",
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
        title: "Deployment created",
        description: `Job ${result.data.id} was accepted by the backend.`,
        variant: "success",
      });

      startTransition(() => {
        router.push(`/${locale}/deployments/${result.data.id}`);
      });
    } catch (caughtError) {
      setFormError(
        caughtError instanceof Error
          ? caughtError.message
          : "Failed to create deployment."
      );
    } finally {
      setCreateLoading(false);
    }
  };

  return (
    <PageStack>
      <PageHeader
        title="Create Deployment"
        description="Build a deployment request only from fields that are confirmed in the frontend's current public HTTP contract."
      />

      <NoticeBanner
        title="Partial contract visibility"
        description="This UI only collects confirmed HTTP fields: `policy_id`, optional `agent_ids`, and optional `params`. Target groups, credentials, job type, and other deployment-plane fields are not exposed here until the public HTTP schema is finalized."
      />

      <SectionCard
        title="Request Builder"
        description="Uses `GET /api/v1/policies`, `POST /api/v1/deployments/plan`, and `POST /api/v1/deployments`."
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
              Preview Plan
            </Button>
            <Button
              size="sm"
              className="h-10 px-4"
              loading={createLoading}
              onClick={() => void handleCreate()}
              disabled={planLoading}
            >
              Create Deployment
            </Button>
          </div>
        }
      >
        <div className="space-y-5">
          <div className="space-y-2">
            <FormLabel>Policy</FormLabel>
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
                title="Policies lookup failed"
                error={policiesQuery.error}
                retry={() => void policiesQuery.refetch()}
              />
            ) : (
              <RequestMetaLine meta={policiesQuery.meta} />
            )}
          </div>

          <TextAreaField
            id="agent_ids"
            label="Agent IDs (optional)"
            helperText="One agent id per line or comma-separated. These are sent only when provided."
            value={agentIdsText}
            onChange={(event) => setAgentIdsText(event.target.value)}
            placeholder={"agent-01\nagent-02"}
          />

          <TextAreaField
            id="params_json"
            label="Params JSON (optional)"
            helperText="Provide a flat JSON object. Values will be serialized to strings, matching the current documented HTTP shape."
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
            title="Request Preview"
            description="This is the exact payload assembled from known fields."
            className="border border-[color:var(--border)] bg-[color:var(--surface)]"
          >
            <JsonPreview value={serializePayloadPreview(requestPayloadPreview)} />
          </SectionCard>
        </div>
      </SectionCard>

      <SectionCard
        title="Plan Response"
        description="The preview result is shown exactly as returned by the backend."
      >
        {planLoading ? (
          <div className="text-sm text-[color:var(--muted-foreground)]">
            Loading plan preview...
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
            Run a plan preview to inspect the backend response.
          </div>
        )}
      </SectionCard>
    </PageStack>
  );
}

function parseDraftPayload(input: {
  policyId: string;
  agentIdsText: string;
  paramsText: string;
}):
  | { payload: DeploymentMutationPayload; error?: undefined; field?: undefined }
  | { payload?: undefined; error: string; field?: "policy" | "params" } {
  const policyId = input.policyId.trim();
  if (!policyId) {
    return {
      error: "Select a policy before previewing or creating a deployment.",
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
          error: "Params must be a JSON object.",
          field: "params",
        };
      }

      params = Object.fromEntries(
        Object.entries(parsed).map(([key, value]) => [key, String(value)])
      );
    } catch {
      return {
        error: "Params must be valid JSON.",
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
