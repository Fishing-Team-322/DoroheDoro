import { PolicyDetailsPage } from "@/src/features/operations";

export default async function DashboardPolicyDetailsRoute({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;

  return <PolicyDetailsPage id={id} />;
}
