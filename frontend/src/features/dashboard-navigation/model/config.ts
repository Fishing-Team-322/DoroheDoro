export type DashboardNavItemKey =
  | "overview"
  | "system"
  | "policies"
  | "deployments"
  | "agents"
  | "logs"
  | "live-logs"
  | "hosts"
  | "host-groups"
  | "credentials"
  | "profile";

export type DashboardNavItem = {
  key: DashboardNavItemKey;
  href: string;
  label: string;
  description?: string;
};

export const dashboardNavigation: DashboardNavItem[] = [
  {
    key: "overview",
    href: "/overview",
    label: "Overview",
    description: "Operational summary",
  },
  {
    key: "system",
    href: "/system",
    label: "System",
    description: "Health, readiness, and auth context",
  },
  {
    key: "policies",
    href: "/policies",
    label: "Policies",
    description: "Policy catalog and details",
  },
  {
    key: "deployments",
    href: "/deployments",
    label: "Deployments",
    description: "Jobs, details, and actions",
  },
  {
    key: "agents",
    href: "/agents",
    label: "Agents",
    description: "Agent registry and diagnostics",
  },
  {
    key: "logs",
    href: "/logs",
    label: "Logs",
    description: "Search and analytics",
  },
  {
    key: "live-logs",
    href: "/logs/live",
    label: "Live Logs",
    description: "SSE tail",
  },
  {
    key: "hosts",
    href: "/hosts",
    label: "Hosts",
    description: "Future inventory management",
  },
  {
    key: "host-groups",
    href: "/host-groups",
    label: "Host Groups",
    description: "Future grouping workflow",
  },
  {
    key: "credentials",
    href: "/credentials",
    label: "Credentials",
    description: "Future credential metadata",
  },
  {
    key: "profile",
    href: "/profile",
    label: "Profile",
    description: "Current account and session settings",
  },
] as const;
