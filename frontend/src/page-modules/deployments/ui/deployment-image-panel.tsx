"use client";

import { Badge, Card } from "@/src/shared/ui";
import {
  DetailGrid,
  StatusBadge,
} from "@/src/features/operations/ui/operations-ui";
import type { DeploymentImageFlow } from "@/src/shared/lib/operations-workbench";

export function DeploymentImagePanel({
  imageFlow,
}: {
  imageFlow: DeploymentImageFlow;
}) {
  return (
    <div className="space-y-4">
      <DetailGrid
        items={[
          { label: "Install mode", value: imageFlow.installMode },
          { label: "Rollout state", value: imageFlow.rolloutState },
          { label: "Image", value: imageFlow.imageLabel },
          { label: "Targets", value: String(imageFlow.affectedTargets) },
          { label: "Succeeded", value: String(imageFlow.succeededTargets) },
          { label: "Failed", value: String(imageFlow.failedTargets) },
        ]}
      />

      <Card className="space-y-3 p-4">
        <h3 className="text-sm font-semibold uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
          Deployment image panel
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
