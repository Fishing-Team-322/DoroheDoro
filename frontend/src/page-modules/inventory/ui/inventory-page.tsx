"use client";

import { useState } from "react";
import { hosts } from "@/src/mocks/data/dashboard";
import { formatPercent, formatRelativeLabel } from "@/src/shared/lib/dashboard";
import { cn } from "@/src/shared/lib/cn";
import { useI18n } from "@/src/shared/lib/i18n";
import { Button, HealthBadge, KeyValueList, StatusBadge } from "@/src/shared/ui";
import type { Host } from "@/src/shared/types/dashboard";
import { HostsTable } from "@/src/widgets/hosts-table";

const CONTENT_INSET = "px-4 md:px-6";
const SECTION_Y = "py-5";

export function InventoryPage() {
  const { dictionary, locale } = useI18n();
  const copy = dictionary.inventory;
  const [selectedHost, setSelectedHost] = useState<Host | undefined>(hosts[0]);

  return (
    <div className="min-w-0">
      <header
        className={cn(
          CONTENT_INSET,
          "border-b border-[color:var(--border)] pb-5 pt-4 md:pb-6 md:pt-16"
        )}
      >
        <nav aria-label="Breadcrumb">
          <ol className="flex flex-wrap items-center gap-2 text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
            <li className="flex items-center gap-2">
              <span>{dictionary.common.dashboard}</span>
              <span className="text-[color:var(--border-strong)]">/</span>
            </li>
            <li className="text-[color:var(--foreground)]">{copy.title}</li>
          </ol>
        </nav>

        <div className="mt-4 flex min-w-0 flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
          <div className="min-w-0 max-w-4xl space-y-2">
            <h1 className="text-[28px] font-semibold tracking-tight text-[color:var(--foreground)]">
              {copy.title}
            </h1>
            <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
              {copy.description}
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2 lg:justify-end">
            <Button type="button" variant="outline">
              {copy.addFilters}
            </Button>
          </div>
        </div>
      </header>

      <div className="grid min-w-0 xl:grid-cols-[minmax(0,1fr)_420px]">
        <div className="min-w-0">
          <section className="min-w-0">
            <HostsTable
              hosts={hosts}
              selectedHostId={selectedHost?.id}
              onSelectHost={setSelectedHost}
            />
          </section>
        </div>

        <aside className="min-w-0 border-t border-[color:var(--border)] xl:border-l xl:border-t-0">
          {selectedHost ? (
            <>
              <section
                className={cn(
                  CONTENT_INSET,
                  SECTION_Y,
                  "border-b border-[color:var(--border)]"
                )}
              >
                <div className="space-y-3">
                  <div className="space-y-1">
                    <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                      {selectedHost.name}
                    </h2>
                    <p className="text-sm text-[color:var(--muted-foreground)]">
                      {selectedHost.cluster} · {selectedHost.os}
                    </p>
                  </div>

                  <div className="flex flex-wrap items-center gap-2">
                    <StatusBadge status={selectedHost.status} />
                    <HealthBadge health={selectedHost.health} />
                  </div>
                </div>
              </section>

              <section
                className={cn(
                  CONTENT_INSET,
                  SECTION_Y,
                  "border-b border-[color:var(--border)]"
                )}
              >
                <div className="mb-4">
                  <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                    {copy.sections.context}
                  </p>
                </div>

                <KeyValueList
                  items={[
                    {
                      label: copy.fields.environment,
                      value: selectedHost.environment.toUpperCase(),
                    },
                    { label: copy.fields.provider, value: selectedHost.provider },
                    { label: copy.fields.region, value: selectedHost.region },
                    { label: copy.fields.policies, value: selectedHost.policyCount },
                  ]}
                />
              </section>

              <section
                className={cn(
                  CONTENT_INSET,
                  SECTION_Y,
                  "border-b border-[color:var(--border)]"
                )}
              >
                <div className="mb-4">
                  <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                    {copy.sections.load}
                  </p>
                </div>

                <KeyValueList
                  items={[
                    { label: copy.fields.ipAddress, value: selectedHost.ipAddress },
                    { label: copy.fields.cpu, value: formatPercent(selectedHost.cpuLoad) },
                    {
                      label: copy.fields.memory,
                      value: formatPercent(selectedHost.memoryUsage),
                    },
                    {
                      label: copy.fields.lastSeen,
                      value: formatRelativeLabel(selectedHost.lastSeenAt, locale),
                    },
                  ]}
                />
              </section>

              <section className={cn(CONTENT_INSET, SECTION_Y)}>
                <div className="mb-4">
                  <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                    {copy.sections.relations}
                  </p>
                </div>

                <KeyValueList
                  items={[
                    { label: copy.fields.tags, value: selectedHost.tags.join(", ") },
                    { label: copy.fields.cluster, value: selectedHost.cluster },
                    { label: copy.fields.platform, value: selectedHost.os },
                  ]}
                />
              </section>
            </>
          ) : (
            <>
              <section
                className={cn(
                  CONTENT_INSET,
                  SECTION_Y,
                  "border-b border-[color:var(--border)]"
                )}
              >
                <div className="space-y-1">
                  <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                    {copy.detailsTitle}
                  </h2>
                  <p className="text-sm text-[color:var(--muted-foreground)]">
                    {copy.detailsDescription}
                  </p>
                </div>
              </section>

              <section className={cn(CONTENT_INSET, SECTION_Y)}>
                <div className="mb-4">
                  <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                    {copy.stateTitle}
                  </p>
                </div>

                <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                  {copy.stateDescription}
                </p>
              </section>
            </>
          )}
        </aside>
      </div>
    </div>
  );
}
