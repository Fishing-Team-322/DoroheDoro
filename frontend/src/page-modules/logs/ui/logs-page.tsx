"use client";

import { logRecords } from "@/src/mocks/data/dashboard";
import { Button, ConsolePage } from "@/src/shared/ui";
import { LogExplorer } from "@/src/widgets/log-explorer";

export function LogsPage() {
  return (
    <ConsolePage>
      <div className="min-w-0">
        <header className="space-y-4">
          <nav aria-label="Breadcrumb">
            <ol className="flex flex-wrap items-center gap-2 text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
              <li className="flex items-center gap-2">
                <span className="text-[color:var(--foreground)]">Панель</span>
                <span className="text-[color:var(--border-strong)]">/</span>
              </li>
              <li className="text-[color:var(--foreground)]">Журналы</li>
            </ol>
          </nav>

          <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div className="max-w-4xl space-y-2">
              <h1 className="text-[28px] font-semibold tracking-tight text-[color:var(--foreground)]">
                Журналы
              </h1>
              <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
                Консоль журналов собрана как query region + split view: слева поток,
                справа structured inspector.
              </p>
            </div>

            <div className="flex flex-wrap items-center gap-2 lg:justify-end">
              <Button type="button" variant="outline">
                Открыть поток
              </Button>
            </div>
          </div>
        </header>

        <section className="min-w-0 border-t border-[color:var(--border)] py-5">
          <LogExplorer records={logRecords} />
        </section>
      </div>
    </ConsolePage>
  );
}
