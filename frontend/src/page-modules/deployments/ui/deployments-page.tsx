"use client";

import { Button, Card } from "@/src/shared/ui";
import { deploymentJobs } from "@/src/mocks/data/dashboard";
import { countJobsByStatus } from "@/src/shared/lib/dashboard";
import { ActivityFeed, SectionHeader, StatCard } from "@/src/shared/ui";
import { PageHeader, Section } from "@/src/widgets/dashboard-layout";
import { JobsTable } from "@/src/widgets/jobs-table";

export function DeploymentsPage() {
  const summary = countJobsByStatus(deploymentJobs);

  return (
    <div className="space-y-6">
      <PageHeader
        title="Развёртывания"
        description="Демонстрационная панель релизов со списком заданий, статусами и недавней активностью."
        action={
          <Button type="button" variant="outline">
            Новый релиз
          </Button>
        }
        breadcrumbs={[{ label: "Панель" }, { label: "Развёртывания" }]}
      />

      <section className="grid gap-4 md:grid-cols-4">
        <StatCard title="Активные задания" value={String(summary.running)} description="Релизы, которые выполняются сейчас" />
        <StatCard title="Успешно" value={String(summary.success)} description="Завершены без ручного вмешательства" />
        <StatCard title="С ошибкой" value={String(summary.failed)} description="Требуют проверки оператором" />
        <StatCard title="В очереди" value={String(summary.pending)} description="Ожидают запуска" />
      </section>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)]">
        <Section className="space-y-4">
          <SectionHeader title="Конвейер релизов" description="Переиспользуемая таблица для сценариев развёртывания." />
          <JobsTable jobs={deploymentJobs} />
        </Section>

        <div className="space-y-4">
          <ActivityFeed
            items={[
              {
                id: "deploy-activity-1",
                title: "Релиз edge-gateway выполняется",
                description: "Успешно завершено 11 из 18 целей.",
                timestamp: deploymentJobs[0].startedAt,
              },
              {
                id: "deploy-activity-2",
                title: "Рекомендуется откат payments",
                description: "Всплеск ошибок совпал с последним неудачным релизом.",
                timestamp: deploymentJobs[1].startedAt,
              },
              {
                id: "deploy-activity-3",
                title: "Развёртывание collector завершено",
                description: "Релиз на стейдже завершился за восемь минут.",
                timestamp: deploymentJobs[2].startedAt,
              },
            ]}
          />
          <Card className="space-y-4">
            <SectionHeader title="Заметки оператора" />
            <p className="text-sm text-zinc-500">
              Этот блок подходит для согласований, действий отката, пометок к релизу или метаданных CI.
            </p>
          </Card>
        </div>
      </div>
    </div>
  );
}

