export type DashboardNavItemKey =
  | "overview"
  | "inventory"
  | "policies"
  | "credentials"
  | "deployments"
  | "agents"
  | "logs"
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
  { key: "inventory", href: "/inventory", fallbackLabel: "Inventory" },
  { key: "policies", href: "/policies", fallbackLabel: "Policies" },
  { key: "credentials", href: "/credentials", fallbackLabel: "Credentials" },
  { key: "deployments", href: "/deployments", fallbackLabel: "Deployments" },
  { key: "agents", href: "/agents", fallbackLabel: "Agents" },
  { key: "logs", href: "/logs", fallbackLabel: "Logs" },
  { key: "alerts", href: "/alerts", fallbackLabel: "Alerts" },
  { key: "audit", href: "/audit", fallbackLabel: "Audit" },
  { key: "profile", href: "/profile", fallbackLabel: "Profile" },
] as const;
