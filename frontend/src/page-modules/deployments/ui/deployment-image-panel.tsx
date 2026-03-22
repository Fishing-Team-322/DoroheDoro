"use client";

import type { Locale } from "@/src/shared/config";
import { Badge, Card } from "@/src/shared/ui";
import {
  DetailGrid,
  StatusBadge,
} from "@/src/features/operations/ui/operations-ui";
import type { DeploymentImageFlow } from "@/src/shared/lib/operations-workbench";

const copyByLocale = {
  en: {
    fields: {
      installMode: "Install mode",
      rolloutState: "Rollout state",
      image: "Image",
      targets: "Targets",
      succeeded: "Succeeded",
      failed: "Failed",
    },
    panelTitle: "Deployment image panel",
  },
  ru: {
    fields: {
      installMode: "Режим установки",
      rolloutState: "Состояние rollout",
      image: "Образ",
      targets: "Таргеты",
      succeeded: "Успешно",
      failed: "Ошибки",
    },
    panelTitle: "Панель образа раскатки",
  },
} as const;

export function DeploymentImagePanel({
  imageFlow,
  locale,
}: {
  imageFlow: DeploymentImageFlow;
  locale: Locale;
}) {
  const copy = copyByLocale[locale];

  return (
    <div className="space-y-4">
      <DetailGrid
        items={[
          { label: copy.fields.installMode, value: imageFlow.installMode },
          { label: copy.fields.rolloutState, value: imageFlow.rolloutState },
          { label: copy.fields.image, value: imageFlow.imageLabel },
          { label: copy.fields.targets, value: String(imageFlow.affectedTargets) },
          { label: copy.fields.succeeded, value: String(imageFlow.succeededTargets) },
          { label: copy.fields.failed, value: String(imageFlow.failedTargets) },
        ]}
      />

      <Card className="space-y-3 p-4">
        <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
          {copy.panelTitle}
        </h3>
        <p className="text-sm text-[color:var(--foreground)]">{imageFlow.imageSource}</p>
      </Card>

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        {imageFlow.phases.map((phase) => (
          <Card key={phase.key} className="space-y-3 p-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="text-sm font-semibold text-[color:var(--foreground)]">
                {phase.label}
              </p>
              <StatusBadge value={phase.status} />
            </div>
            <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
              {phase.detail}
            </p>
            <Badge>{phase.key}</Badge>
          </Card>
        ))}
      </div>
    </div>
  );
}
