import { UnavailablePage } from "@/src/features/operations";

export default function DashboardHostGroupsRoute() {
  return (
    <UnavailablePage
      title="Host Groups"
      description="Host group management is not available through the current public HTTP API yet."
    />
  );
}
