"use client";

import { useEffect, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
import {
  getAgentDiagnostics,
  listAgents,
  type AgentDiagnosticsItem,
  type AgentItem,
} from "@/src/shared/lib/runtime-api";
import {
  Card,
  EmptyState,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import {
  ErrorCard,
  JsonValue,
  LoadingCard,
} from "@/src/page-modules/common/ui/runtime-state";

export function AgentsPage() {
  const { dictionary } = useI18n();
  const [agents, setAgents] = useState<AgentItem[]>([]);
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<AgentDiagnosticsItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [diagnosticsLoading, setDiagnosticsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const response = await listAgents();
        if (!cancelled) {
          setAgents(response.items);
          setSelectedAgentId((current) => current ?? response.items[0]?.agent_id ?? null);
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(loadError instanceof Error ? loadError.message : "Failed to load agents");
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void load();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function loadDiagnostics() {
      if (!selectedAgentId) {
        setDiagnostics([]);
        return;
      }
      setDiagnosticsLoading(true);
      try {
        const response = await getAgentDiagnostics(selectedAgentId);
        if (!cancelled) {
          setDiagnostics(response.items);
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error
              ? loadError.message
              : "Failed to load diagnostics"
          );
        }
      } finally {
        if (!cancelled) {
          setDiagnosticsLoading(false);
        }
      }
    }

    void loadDiagnostics();
    return () => {
      cancelled = true;
    };
  }, [selectedAgentId]);

  const selectedAgent = agents.find((item) => item.agent_id === selectedAgentId) ?? null;

  return (
    <div className="space-y-6">
      <PageHeader
        title="Agents"
        description="Live agent registry, status and diagnostics from enrollment-plane."
        breadcrumbs={[
          { label: dictionary.common.dashboard, href: "#" },
          { label: "Agents" },
        ]}
      />

      {loading ? <LoadingCard label="Loading agents..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
          <Card>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Host</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Version</TableHead>
                  <TableHead>Last seen</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {agents.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={4}>
                      <EmptyState
                        variant="flush"
                        title="No enrolled agents"
                        description="Enroll agents to populate the registry."
                      />
                    </TableCell>
                  </TableRow>
                ) : (
                  agents.map((agent) => (
                    <TableRow
                      key={agent.agent_id}
                      className={
                        agent.agent_id === selectedAgentId
                          ? "bg-[color:rgba(56,189,248,0.08)]"
                          : undefined
                      }
                      onClick={() => setSelectedAgentId(agent.agent_id)}
                    >
                      <TableCell className="font-medium text-[color:var(--foreground)]">
                        {agent.hostname}
                      </TableCell>
                      <TableCell>{agent.status}</TableCell>
                      <TableCell>{agent.version || "unknown"}</TableCell>
                      <TableCell>{agent.last_seen_at}</TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </Card>

          <Card>
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                Agent inspector
              </h2>
              {selectedAgent ? (
                <>
                  <div className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3">
                    <p className="text-lg font-semibold text-[color:var(--foreground)]">
                      {selectedAgent.hostname}
                    </p>
                    <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                      {selectedAgent.agent_id}
                    </p>
                  </div>
                  <JsonValue value={selectedAgent.metadata_json} />

                  <div className="space-y-2">
                    <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      Latest diagnostics
                    </h3>
                    {diagnosticsLoading ? (
                      <LoadingCard label="Loading diagnostics..." />
                    ) : diagnostics[0] ? (
                      <JsonValue value={diagnostics[0].payload_json} />
                    ) : (
                      <EmptyState
                        variant="flush"
                        title="No diagnostics yet"
                        description="Diagnostics appear after the agent reports its runtime state."
                      />
                    )}
                  </div>
                </>
              ) : (
                <EmptyState
                  variant="flush"
                  title="No agent selected"
                  description="Pick an agent to inspect its metadata and diagnostics."
                />
              )}
            </div>
          </Card>
        </section>
      ) : null}
    </div>
  );
}
