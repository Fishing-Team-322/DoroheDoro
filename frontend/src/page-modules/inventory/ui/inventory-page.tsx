"use client";

import { useState } from "react";
import { hosts } from "@/src/mocks/data/dashboard";
import { formatPercent, formatRelativeLabel } from "@/src/shared/lib/dashboard";
import { cn } from "@/src/shared/lib/cn";
import {
  Button,
  ConsolePage,
  HealthBadge,
  KeyValueList,
  StatusBadge,
} from "@/src/shared/ui";
import type { Host } from "@/src/shared/types/dashboard";
import { HostsTable } from "@/src/widgets/hosts-table";

const CONTENT_INSET = "px-4 md:px-6";
const SECTION_Y = "py-5";

export function InventoryPage() {
  const [selectedHost, setSelectedHost] = useState<Host | undefined>(hosts[0]);

  return (
    <ConsolePage>
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
                <span>Панель</span>
                <span className="text-[color:var(--border-strong)]">/</span>
              </li>
              <li className="text-[color:var(--foreground)]">Инвентарь</li>
            </ol>
          </nav>

          <div className="mt-4 flex min-w-0 flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div className="min-w-0 max-w-4xl space-y-2">
              <h1 className="text-[28px] font-semibold tracking-tight text-[color:var(--foreground)]">
                Инвентарь
              </h1>
              <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                Table-first экран для управляемых хостов, фильтрации и анализа операционного
                состояния.
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-2 lg:justify-end">
              <Button type="button" variant="outline">
                Добавить набор фильтров
              </Button>
            </div>
          </div>
        </header>

        <div className="grid min-w-0 xl:grid-cols-[minmax(0,1fr)_420px]">
          <main className="min-w-0">
            <section className="min-w-0">
              <HostsTable
                hosts={hosts}
                selectedHostId={selectedHost?.id}
                onSelectHost={setSelectedHost}
              />
            </section>
          </main>

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
                      Контекст
                    </p>
                  </div>

                  <KeyValueList
                    items={[
                      { label: "Окружение", value: selectedHost.environment.toUpperCase() },
                      { label: "Провайдер", value: selectedHost.provider },
                      { label: "Регион", value: selectedHost.region },
                      { label: "Политики", value: selectedHost.policyCount },
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
                      Нагрузка
                    </p>
                  </div>

                  <KeyValueList
                    items={[
                      { label: "IP-адрес", value: selectedHost.ipAddress },
                      { label: "Загрузка CPU", value: formatPercent(selectedHost.cpuLoad) },
                      {
                        label: "Использование памяти",
                        value: formatPercent(selectedHost.memoryUsage),
                      },
                      {
                        label: "Последний сигнал",
                        value: formatRelativeLabel(selectedHost.lastSeenAt),
                      },
                    ]}
                  />
                </section>

                <section className={cn(CONTENT_INSET, SECTION_Y)}>
                  <div className="mb-4">
                    <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                      Теги и связи
                    </p>
                  </div>

                  <KeyValueList
                    items={[
                      { label: "Теги", value: selectedHost.tags.join(", ") },
                      { label: "Кластер", value: selectedHost.cluster },
                      { label: "Платформа", value: selectedHost.os },
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
                      Детали хоста
                    </h2>
                    <p className="text-sm text-[color:var(--muted-foreground)]">
                      Выберите хост в таблице, чтобы открыть инспектор.
                    </p>
                  </div>
                </section>

                <section className={cn(CONTENT_INSET, SECTION_Y)}>
                  <div className="mb-4">
                    <p className="text-xs font-semibold uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                      Состояние
                    </p>
                  </div>

                  <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                    Правая колонка показывает операционный контекст, сигналы и нагрузку выбранного
                    узла без перехода на отдельный экран.
                  </p>
                </section>
              </>
            )}
          </aside>
        </div>
      </div>
    </ConsolePage>
  );
}