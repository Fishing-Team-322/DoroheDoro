import { DeploymentDetailsPage } from "@/src/features/operations";

export default async function DashboardDeploymentDetailsRoute({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;

  return <DeploymentDetailsPage id={id} />;
}
