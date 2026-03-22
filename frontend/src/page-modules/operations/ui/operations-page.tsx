"use client";

import Link from "next/link";
import { useMemo } from "react";
import { useSearchParams } from "next/navigation";
import { DeploymentsPage } from "@/src/page-modules/deployments";
import { LogsPage } from "@/src/page-modules/logs";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { Button, Card } from "@/src/shared/ui";

const copyByLocale = {
  en: {
    tabs: {
      deployments: "Deployments",
      logs: "Logs",
    },
    title: "Operations workspace",
    liveLogs: "Open Live Logs",
  },
  ru: {
    tabs: {
      deployments: "Раскатки",
      logs: "Логи",
    },
    title: "операции/логи",
    liveLogs: "Открыть live-логи",
  },
} as const;

type OperationsTab = "deployments" | "logs";

const validTabs: OperationsTab[] = ["deployments", "logs"];

function getActiveTab(value: string | null): OperationsTab {
  return validTabs.includes(value as OperationsTab)
    ? (value as OperationsTab)
    : "deployments";
}

export function OperationsPage() {
  const { locale } = useI18n();
  const copy = copyByLocale[locale];
  const searchParams = useSearchParams();
  const activeTab = getActiveTab(searchParams.get("tab"));

  const tabs = useMemo(
    () => [
      {
        id: "deployments" as const,
        label: copy.tabs.deployments,
        href: withLocalePath(locale, "/operations?tab=deployments"),
      },
      {
        id: "logs" as const,
        label: copy.tabs.logs,
        href: withLocalePath(locale, "/operations?tab=logs"),
      },
    ],
    [copy.tabs.deployments, copy.tabs.logs, locale]
  );

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="space-y-6">
          <div className="flex flex-col gap-4 border-b border-[color:var(--border)] pb-6 xl:flex-row xl:items-center xl:justify-between">
            <div className="space-y-2">
              <h2 className="text-5xl font-semibold text-[color:var(--foreground)]">
                {copy.title}
              </h2>
            </div>

            <div className="flex flex-wrap items-center gap-3">
              {activeTab === "logs" ? (
                <Link href={withLocalePath(locale, "/logs/live")}>
                  <Button variant="outline" size="sm" className="h-9 px-4">
                    {copy.liveLogs}
                  </Button>
                </Link>
              ) : null}

              <div className="inline-flex rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-1 gap-1">
                {tabs.map((tab) => {
                  const isActive = tab.id === activeTab;

                  return (
                    <Link key={tab.id} href={tab.href}>
                      <Button
                        variant={isActive ? "default" : "ghost"}
                        size="sm"
                        className="h-9 px-4"
                      >
                        {tab.label}
                      </Button>
                    </Link>
                  );
                })}
              </div>
            </div>
          </div>

          <div>
            {activeTab === "deployments" ? <DeploymentsPage embedded /> : null}
            {activeTab === "logs" ? <LogsPage embedded /> : null}
          </div>
        </div>
      </Card>
    </div>
  );
}
