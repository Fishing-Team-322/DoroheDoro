"use client";

import { getAuthContext, getHealthStatus, getReadinessStatus } from "../api";
import { useApiQuery } from "../model";
import { ErrorState, LoadingState, PageStack } from "./operations-ui";

function StatusValue({
  children,
  tone = "neutral",
}: {
  children: React.ReactNode;
  tone?: "success" | "warning" | "error" | "neutral";
}) {
  const toneClass =
    tone === "success"
      ? "text-emerald-400"
      : tone === "warning"
        ? "text-amber-400"
        : tone === "error"
          ? "text-red-400"
          : "text-[color:var(--foreground)]";

  return <span className={`font-semibold ${toneClass}`}>{children}</span>;
}

function InfoRow({
  label,
  value,
}: {
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div className="grid grid-cols-1 gap-2 py-4 md:grid-cols-[260px_minmax(0,1fr)] md:gap-6">
      <div className="text-[15px] text-[color:var(--muted-foreground)] md:text-[16px]">
        {label}
      </div>
      <div className="text-[15px] font-semibold text-[color:var(--foreground)] md:text-[16px]">
        {value}
      </div>
    </div>
  );
}

function formatHealthStatus(value?: string, hasError?: boolean) {
  if (hasError) return { text: "ERROR", tone: "error" as const };
  if (!value) return { text: "UNKNOWN", tone: "neutral" as const };

  const normalized = value.toLowerCase();

  if (normalized === "ok") {
    return { text: "OK", tone: "success" as const };
  }

  if (normalized === "ready") {
    return { text: "READY", tone: "success" as const };
  }

  return { text: value.toUpperCase(), tone: "neutral" as const };
}

export function SystemStatusPage() {
  const healthQuery = useApiQuery({
    queryFn: getHealthStatus,
    deps: [],
  });

  const readinessQuery = useApiQuery({
    queryFn: getReadinessStatus,
    deps: [],
  });

  const authQuery = useApiQuery({
    queryFn: getAuthContext,
    deps: [],
  });

  const isInitialLoading =
    (healthQuery.isLoading && !healthQuery.data) ||
    (readinessQuery.isLoading && !readinessQuery.data) ||
    (authQuery.isLoading && !authQuery.data);

  const criticalError =
    (healthQuery.error && !healthQuery.data) ||
    (readinessQuery.error && !readinessQuery.data) ||
    (authQuery.error && !authQuery.data);

  const health = formatHealthStatus(
    healthQuery.data?.status,
    Boolean(healthQuery.error && !healthQuery.data),
  );

  const readiness = formatHealthStatus(
    readinessQuery.data?.status,
    Boolean(readinessQuery.error && !readinessQuery.data),
  );

  const subject = authQuery.data?.user?.subject ?? "—";
  const role = authQuery.data?.user?.role ?? "—";
  const authMode = authQuery.data?.auth?.mode ?? "—";

  const firstError =
    (healthQuery.error && !healthQuery.data
      ? healthQuery.error
      : readinessQuery.error && !readinessQuery.data
        ? readinessQuery.error
        : authQuery.error) ?? undefined;

  return (
    <PageStack>
      <div className="rounded-[28px] border border-[color:var(--border)] bg-[color:var(--surface)] p-8 md:p-10">
        {isInitialLoading ? (
          <LoadingState compact label="Загружаем системную информацию..." />
        ) : criticalError ? (
          <ErrorState
            error={firstError}
            retry={() => {
              void healthQuery.refetch();
              void readinessQuery.refetch();
              void authQuery.refetch();
            }}
          />
        ) : (
          <div className="space-y-8">
            <div className="space-y-3">
              <h2 className="text-3xl font-semibold tracking-tight text-[color:var(--foreground)] md:text-5xl">
                состояние системы
              </h2>
            </div>

            <div className="border-t border-[color:var(--border)] pt-2">
              <InfoRow
                label="публичный API"
                value={<StatusValue tone={health.tone}>{health.text}</StatusValue>}
              />
              <InfoRow
                label="backend"
                value={<StatusValue tone={readiness.tone}>{readiness.text}</StatusValue>}
              />
              <InfoRow label="пользователь текущей сессии" value={subject} />
              <InfoRow label="роль" value={role} />
              <InfoRow label="режим авторизации" value={authMode} />
            </div>

            <div className="border-t border-[color:var(--border)] pt-6">
              <p className="max-w-3xl text-sm leading-7 text-[color:var(--muted-foreground)] md:text-base">
                Эта страница помогает быстро понять, работает ли системная часть платформы и под
                каким контекстом открыта текущая сессия. Если другие разделы недоступны, сначала
                проверьте именно эти статусы.
              </p>
            </div>
          </div>
        )}
      </div>
    </PageStack>
  );
}
