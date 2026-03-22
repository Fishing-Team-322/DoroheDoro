"use client";

import Link from "next/link";
import { useMemo } from "react";
import { useSearchParams } from "next/navigation";
import { DeploymentsPage } from "@/src/page-modules/deployments";
import { LogsPage } from "@/src/page-modules/logs";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { Button, Card } from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";

type OperationsTab = "deployments" | "logs";

const validTabs: OperationsTab[] = ["deployments", "logs"];

function getActiveTab(value: string | null): OperationsTab {
  return validTabs.includes(value as OperationsTab)
    ? (value as OperationsTab)
    : "deployments";
}

export function OperationsPage() {
  const { locale } = useI18n();
  const searchParams = useSearchParams();
  const activeTab = getActiveTab(searchParams.get("tab"));

  const tabs = useMemo(
    () => [
      {
        id: "deployments" as const,
        label: "Deployments",
        href: withLocalePath(locale, "/operations?tab=deployments"),
      },
      {
        id: "logs" as const,
        label: "Logs",
        href: withLocalePath(locale, "/operations?tab=logs"),
      },
    ],
    [locale]
  );

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="space-y-6">
          <div className="flex flex-col gap-4 border-b border-[color:var(--border)] pb-6 xl:flex-row xl:items-center xl:justify-between">
            <div className="space-y-2">
              <h2 className="text-5xl font-semibold text-[color:var(--foreground)]">
                operations workspace
              </h2>
            </div>

            <div className="flex flex-wrap items-center gap-3">
              {activeTab === "logs" ? (
                <Link href={withLocalePath(locale, "/logs/live")}>
                  <Button variant="outline" size="sm" className="h-9 px-4">
                    Open Live Logs
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