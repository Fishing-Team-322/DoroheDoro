"use client";

import { useEffect, useMemo, useState } from "react";
import { createPortal } from "react-dom";
import {
  DetailGrid,
  NoticeBanner,
  StatusBadge,
  TextAreaField,
} from "@/src/features/operations/ui/operations-ui";
import type { PolicySummary } from "@/src/shared/lib/runtime-api";
import { Badge, Button, Card, FormLabel, Input, Select } from "@/src/shared/ui";

const BOOTSTRAP_TOKEN_PLACEHOLDER =
  "Unavailable until the public Edge API bridge is exposed";

export function AgentEnrollmentDialog({
  open,
  onClose,
  policies,
  initialPolicyId,
}: {
  open: boolean;
  onClose: () => void;
  policies: PolicySummary[];
  initialPolicyId?: string | null;
}) {
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
                    Create Agent
                  </h2>
                  <Badge variant="warning">UI stub</Badge>
                </div>

                <p className="max-w-3xl text-sm leading-6 text-[color:var(--muted-foreground)]">
                  Prepare the future enrollment payload using only data already
                  loaded in WEB. No create or enrollment request is sent from
                  this dialog.
                </p>
              </div>

              <Button
                variant="outline"
                size="sm"
                className="h-10 px-4"
                onClick={onClose}
              >
                Close
              </Button>
            </div>

            <NoticeBanner
              title="Public Edge API bridge required"
              description="Real agent create and enrollment flows will become available only after Edge exposes a public HTTP bridge for bootstrap token issuance and enrollment."
            />

            <div className="grid gap-4 md:grid-cols-2">
              <Input
                label="Agent name"
                value={agentName}
                onChange={(event) => setAgentName(event.target.value)}
                helperText="UI-only draft field."
              />

              <Input
                label="Hostname"
                value={hostname}
                onChange={(event) => setHostname(event.target.value)}
                helperText="Preview only. Nothing is persisted."
              />

              <Input
                label="Environment"
                value={environment}
                onChange={(event) => setEnvironment(event.target.value)}
                helperText="For example: prod, staging, lab."
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
                    policies.length > 0 ? "Select policy" : "No policies loaded"
                  }
                  disabled={policies.length === 0}
                />
                <p className="text-sm text-[color:var(--muted-foreground)]">
                  Uses the policies already fetched on this page.
                </p>
              </div>
            </div>

            <TextAreaField
              id="create-agent-labels"
              label="Labels / Tags (optional)"
              helperText="Optional preview field. Use comma-separated or newline-separated values."
              value={labelsText}
              onChange={(event) => setLabelsText(event.target.value)}
              placeholder={"role=edge\nregion=eu-central-1"}
              className="min-h-24"
            />

            <div className="space-y-4 rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
              <div className="space-y-2">
                <p className="text-sm font-semibold text-[color:var(--foreground)]">
                  Policy preview
                </p>
                <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                  Preview is based on the currently loaded policy list and does
                  not call policy creation or bootstrap endpoints.
                </p>
              </div>

              {selectedPolicy ? (
                <DetailGrid
                  items={[
                    {
                      label: "Policy name",
                      value: selectedPolicy.name,
                    },
                    {
                      label: "Policy ID",
                      value: selectedPolicy.id,
                    },
                    {
                      label: "Revision",
                      value: selectedPolicy.revision ?? "n/a",
                    },
                    {
                      label: "Status",
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
                  Load policies or select one to see the preview.
                </p>
              )}

              <div className="space-y-2">
                <p className="text-sm font-medium text-[color:var(--foreground)]">
                  Parsed labels
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
                    No labels added yet.
                  </p>
                )}
              </div>
            </div>

            <div className="grid gap-4 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
              <div className="space-y-4 rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                <Input
                  label="Bootstrap token"
                  value={BOOTSTRAP_TOKEN_PLACEHOLDER}
                  readOnly
                  disabled
                  helperText="Disabled until Edge exposes the public bridge for `agents.bootstrap-token.issue`."
                />

                <Button
                  variant="outline"
                  size="sm"
                  className="h-10 px-4"
                  disabled
                >
                  Issue Bootstrap Token
                </Button>
              </div>

              <div className="space-y-3 rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-4">
                <div>
                  <p className="text-sm font-semibold text-[color:var(--foreground)]">
                    Enrollment command preview
                  </p>
                  <p className="mt-1 text-sm leading-6 text-[color:var(--muted-foreground)]">
                    Preview only. The command remains incomplete until a public
                    bootstrap token bridge exists.
                  </p>
                </div>

                <pre className="overflow-x-auto rounded-lg border border-[color:var(--border)] bg-[color:var(--surface-subtle)] p-3 text-xs leading-6 text-[color:var(--foreground)]">
                  <code>{commandPreview}</code>
                </pre>
              </div>
            </div>

            <div className="flex flex-wrap items-center justify-between gap-3 border-t border-[color:var(--border)] pt-4">
              <p className="max-w-3xl text-sm leading-6 text-[color:var(--muted-foreground)]">
                Real enrollment and create actions stay disabled on purpose
                until a public Edge API bridge is available for WEB.
              </p>

              <div className="flex flex-wrap gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  className="h-10 px-4"
                  onClick={onClose}
                >
                  Cancel
                </Button>
                <Button size="sm" className="h-10 px-4" disabled>
                  Prepare Enrollment
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
