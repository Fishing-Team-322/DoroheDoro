"use client";

import { useState } from "react";
import { Button, Card } from "@/src/shared/ui";
import { alerts } from "@/src/mocks/data/dashboard";
import { countAlertsByStatus, formatRelativeLabel } from "@/src/shared/lib/dashboard";
import { DetailsDrawer, KeyValueList, SectionHeader, SeverityBadge, StatCard, StatusBadge } from "@/src/shared/ui";
import type { Alert } from "@/src/shared/types/dashboard";
import { PageHeader, Section } from "@/src/widgets/dashboard-layout";
import { AlertsTable } from "@/src/widgets/alerts-table";

export function AlertsPage() {
  const [selectedAlert, setSelectedAlert] = useState<Alert | undefined>(alerts[0]);
  const summary = countAlertsByStatus(alerts);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Оповещения"
        description="Консоль инцидентов с карточками сводки, индикаторами важности и статуса, а также боковой панелью деталей."
        action={
          <Button type="button" variant="outline">
            Создать правило приглушения
          </Button>
        }
        breadcrumbs={[{ label: "Панель" }, { label: "Оповещения" }]}
      />

      <section className="grid gap-4 md:grid-cols-4">
        <StatCard title="Активные" value={String(summary.active)} description="Требуют немедленного действия" />
        <StatCard title="Подтверждённые" value={String(summary.acknowledged)} description="Уже взяты в работу" />
        <StatCard title="Решённые" value={String(summary.resolved)} description="Закрыты в выбранном окне" />
        <StatCard title="Приглушённые" value={String(summary.muted)} description="Подавлены политикой или оператором" />
      </section>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.8fr)_420px]">
        <Section className="space-y-4">
          <SectionHeader title="Очередь инцидентов" description="Готовая таблица для страниц с большим числом оповещений." />
          <AlertsTable
            alerts={alerts}
            selectedAlertId={selectedAlert?.id}
            onSelectAlert={setSelectedAlert}
          />
        </Section>

        <DetailsDrawer
          title={selectedAlert?.title ?? "Детали оповещения"}
          description={selectedAlert ? `${selectedAlert.source} - ${selectedAlert.host}` : "Выберите оповещение"}
          open={true}
        >
          {selectedAlert ? (
            <>
              <Card className="space-y-4">
                <div className="flex flex-wrap items-center gap-2">
                  <SeverityBadge severity={selectedAlert.severity} />
                  <StatusBadge status={selectedAlert.status} />
                </div>
                <p className="text-sm text-zinc-600">{selectedAlert.summary}</p>
              </Card>
              <KeyValueList
                items={[
                  { label: "Источник", value: selectedAlert.source },
                  { label: "Окружение", value: selectedAlert.environment.toUpperCase() },
                  { label: "Хост", value: selectedAlert.host },
                  { label: "Исполнитель", value: selectedAlert.assignee ?? "Не назначен" },
                  { label: "Сработало", value: formatRelativeLabel(selectedAlert.triggeredAt) },
                ]}
              />
            </>
          ) : null}
        </DetailsDrawer>
      </div>
    </div>
  );
}

