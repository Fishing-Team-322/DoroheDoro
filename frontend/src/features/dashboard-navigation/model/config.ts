export type DashboardNavItemKey =
  | "overview"
  | "system"
  | "inventory"
  | "policies"
  | "security"
  | "credentials"
  | "deployments"
  | "anomalies"
  | "integrations"
  | "agents"
  | "logs"
  | "live-logs"
  | "alerts"
  | "audit"
  | "profile";

export type DashboardNavItem = {
  key: DashboardNavItemKey;
  href: string;
  fallbackLabel: string;
};

export const dashboardNavigation: DashboardNavItem[] = [
  { key: "overview", href: "/overview", fallbackLabel: "Overview" },
  { key: "system", href: "/system", fallbackLabel: "System" },
  { key: "inventory", href: "/inventory", fallbackLabel: "Inventory" },
  { key: "policies", href: "/policies", fallbackLabel: "Policies" },
  { key: "security", href: "/security", fallbackLabel: "Security" },
  { key: "credentials", href: "/credentials", fallbackLabel: "Credentials" },
  { key: "deployments", href: "/deployments", fallbackLabel: "Deployments" },
  { key: "anomalies", href: "/anomalies", fallbackLabel: "Anomalies" },
  { key: "integrations", href: "/integrations", fallbackLabel: "Integrations" },
  { key: "agents", href: "/agents", fallbackLabel: "Agents" },
  { key: "logs", href: "/logs", fallbackLabel: "Logs" },
  { key: "live-logs", href: "/logs/live", fallbackLabel: "Live Logs" },
  { key: "alerts", href: "/alerts", fallbackLabel: "Alerts" },
  { key: "audit", href: "/audit", fallbackLabel: "Audit" },
  { key: "profile", href: "/profile", fallbackLabel: "Profile" },
] as const;
