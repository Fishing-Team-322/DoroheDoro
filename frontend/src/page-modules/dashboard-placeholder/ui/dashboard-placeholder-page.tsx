"use client";

import { useI18n } from "@/src/shared/lib/i18n";
import { Button, SectionHeader } from "@/src/shared/ui";

export function DashboardPlaceholderPage() {
  const { dictionary } = useI18n();
  const copy = dictionary.policies;

  return (
    <div className="min-w-0">
      <header className="space-y-4">
        <nav aria-label="Breadcrumb">
          <ol className="flex flex-wrap items-center gap-2 text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
            <li className="flex items-center gap-2">
              <span className="text-[color:var(--foreground)]">
                {dictionary.common.dashboard}
              </span>
              <span className="text-[color:var(--border-strong)]">/</span>
            </li>
            <li className="text-[color:var(--foreground)]">{copy.title}</li>
          </ol>
        </nav>

        <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
          <div className="max-w-4xl space-y-2">
            <h1 className="text-[28px] font-semibold tracking-tight text-[color:var(--foreground)]">
              {copy.title}
            </h1>
            <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
              {copy.description}
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2 lg:justify-end">
            <Button type="button" variant="outline">
              {copy.planModule}
            </Button>
          </div>
        </div>
      </header>

      <section className="min-w-0 border-t border-[color:var(--border)]">
        <div className="grid gap-0 xl:grid-cols-[minmax(0,1.4fr)_360px]">
          <div className="space-y-5 py-5 pr-0 xl:pr-6">
            <SectionHeader
              title={copy.plannedModuleTitle}
              description={copy.plannedModuleDescription}
            />

            <div className="space-y-0">
              <div className="border-b border-[color:var(--border)] py-4 first:border-t">
                <p className="text-sm font-medium text-[color:var(--foreground)]">
                  {copy.statusTitle}
                </p>
                <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                  {copy.statusDescription}
                </p>
              </div>

              <div className="border-b border-[color:var(--border)] py-4">
                <p className="text-sm font-medium text-[color:var(--foreground)]">
                  {copy.whatWillAppearTitle}
                </p>
                <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                  {copy.whatWillAppearDescription}
                </p>
              </div>

              <div className="border-b border-[color:var(--border)] py-4">
                <p className="text-sm font-medium text-[color:var(--foreground)]">
                  {copy.nextStepTitle}
                </p>
                <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                  {copy.nextStepDescription}
                </p>
              </div>
            </div>
          </div>

          <div className="border-t border-[color:var(--border)] py-5 xl:border-l xl:border-t-0 xl:pl-6">
            <SectionHeader
              title={copy.areaSkeletonTitle}
              description={copy.areaSkeletonDescription}
            />
            <div className="mt-4 w-full border-t border-[color:var(--border)]" />

            <div className="space-y-0 pt-4">
              {copy.rows.map((row) => (
                <div
                  key={row.label}
                  className="border-b border-[color:var(--border)] py-3 first:border-t"
                >
                  <p className="text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
                    {row.label}
                  </p>
                  <p className="mt-1 text-sm text-[color:var(--foreground)]">
                    {row.description}
                  </p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
