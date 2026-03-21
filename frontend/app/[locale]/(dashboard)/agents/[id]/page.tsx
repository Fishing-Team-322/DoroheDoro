import { AgentDetailsPage } from "@/src/features/operations";

export default async function DashboardAgentDetailsRoute({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;

  return <AgentDetailsPage id={id} />;
}
