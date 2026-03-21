import { UnavailablePage } from "@/src/features/operations";

export default function DashboardHostsRoute() {
  return (
    <UnavailablePage
      title="Hosts"
      description="Hosts management is not available through the current public HTTP API yet."
    />
  );
}
