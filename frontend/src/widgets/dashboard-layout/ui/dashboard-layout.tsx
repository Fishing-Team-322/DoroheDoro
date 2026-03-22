"use client";

import type { ComponentType, ReactNode } from "react";
import { useMemo, useState } from "react";
import * as Tooltip from "@radix-ui/react-tooltip";
import { motion } from "motion/react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  dashboardNavigation,
  type DashboardNavItem,
} from "@/src/features/dashboard-navigation";
import { useAuth } from "@/src/features/auth/model/use-auth";
import type { Locale } from "@/src/shared/config";
import { cn } from "@/src/shared/lib/cn";
import { useI18n, withLocalePath } from "@/src/shared/lib/i18n";
import { ConsolePage } from "@/src/shared/ui";
import { DashboardSidebarLanguageSwitch } from "@/src/shared/ui/lang-switch";
import {
  ActivityIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  GridIcon,
  HomeIcon,
  MenuIcon,
  ServerIcon,
  ShieldIcon,
  TerminalIcon,
  UserIcon,
} from "../icons";
import { useSidebarCollapsedState } from "../model/use-sidebar-collapsed-state";

type DashboardLayoutProps = {
  locale: Locale;
  children: ReactNode;
};

type NavItem = DashboardNavItem & {
  icon?: ComponentType<{ className?: string }>;
};

type DashboardSidebarProps = {
  locale: Locale;
  collapsed: boolean;
  mobileOpen: boolean;
  onToggle: () => void;
  onCloseMobile: () => void;
};

const SIDEBAR_EXPANDED_WIDTH = 288;
const SIDEBAR_COLLAPSED_WIDTH = 88;
const BOTTOM_CONTROL_SIZE = 40;
const BOTTOM_SWITCH_WIDTH = 78;

const SIDEBAR_TRANSITION = {
  duration: 0.28,
  ease: [0.22, 1, 0.36, 1] as const,
};

export function DashboardLayout({ locale, children }: DashboardLayoutProps) {
  const [collapsed, setCollapsed] = useSidebarCollapsedState();
  const [mobileOpen, setMobileOpen] = useState(false);
  const { dictionary } = useI18n();

  return (
    <div className="min-h-screen bg-[color:var(--background)] text-[color:var(--foreground)]">
      <div className="flex min-h-screen">
        <motion.div
          initial={false}
          animate={{
            width: collapsed ? SIDEBAR_COLLAPSED_WIDTH : SIDEBAR_EXPANDED_WIDTH,
          }}
          transition={SIDEBAR_TRANSITION}
          className="hidden shrink-0 lg:block"
          aria-hidden="true"
        />

        <DashboardSidebar
          locale={locale}
          collapsed={collapsed}
          mobileOpen={mobileOpen}
          onToggle={() => setCollapsed((prev) => !prev)}
          onCloseMobile={() => setMobileOpen(false)}
        />

        <main className="relative min-h-screen min-w-0 flex-1 overflow-y-auto">
          <button
            type="button"
            onClick={() => setMobileOpen(true)}
            aria-label={dictionary.navigation.expandSidebar}
            className="fixed left-4 top-4 z-30 inline-flex h-11 w-11 items-center justify-center rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface-elevated)] text-[color:var(--foreground)] shadow-[0_10px_30px_rgba(0,0,0,0.25)] lg:hidden"
          >
            <MenuIcon className="h-5 w-5" />
          </button>

          <DashboardPageContainer>{children}</DashboardPageContainer>
        </main>
      </div>
    </div>
  );
}

export function DashboardSidebar({
  locale,
  collapsed,
  mobileOpen,
  onToggle,
  onCloseMobile,
}: DashboardSidebarProps) {
  const pathname = usePathname();
  const { dictionary } = useI18n();
  const { user } = useAuth();

  const items = useMemo<NavItem[]>(
    () =>
      dashboardNavigation.map((item) => ({
        ...item,
        icon: getSidebarIcon(item),
      })),
    []
  );

  const username = getSidebarUsername(user);
  const usernameBase = `@${username.slice(0, 2)}`;
  const usernameTail = username.slice(2);

  return (
    <>
      <motion.div
        initial={false}
        animate={{ opacity: mobileOpen ? 1 : 0 }}
        transition={{ duration: 0.18 }}
        onClick={onCloseMobile}
        className={cn(
          "fixed inset-0 z-40 bg-black/50 backdrop-blur-[2px] lg:hidden",
          mobileOpen ? "pointer-events-auto" : "pointer-events-none"
        )}
      />

      <motion.aside
        initial={false}
        animate={{
          width: collapsed ? SIDEBAR_COLLAPSED_WIDTH : SIDEBAR_EXPANDED_WIDTH,
        }}
        transition={SIDEBAR_TRANSITION}
        className={cn(
          "fixed inset-y-0 left-0 z-50 h-screen bg-[color:var(--background)] lg:z-40",
          mobileOpen ? "translate-x-0" : "-translate-x-full lg:translate-x-0"
        )}
      >
        <div className="relative flex h-full min-h-0 flex-col overflow-hidden px-4 py-5">
          <div className="mb-8 shrink-0 overflow-hidden">
            <div
              className="flex h-12 items-center pl-[6px]"
              title={`@${username}`}
            >
              <div className="min-w-0 overflow-hidden">
                <div className="inline-flex h-[24px] items-center whitespace-nowrap text-[20px] font-medium italic leading-[1] tracking-[-0.01em] text-[color:var(--foreground)]">
                  <span className="shrink-0">{usernameBase}</span>

                  <span
                    className={cn(
                      "inline-block -ml-[0.02em] overflow-hidden transition-[max-width,opacity] duration-300 ease-[cubic-bezier(0.22,1,0.36,1)]",
                      collapsed
                        ? "max-w-0 opacity-0"
                        : "max-w-[160px] opacity-100"
                    )}
                  >
                    {usernameTail}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            <nav className="space-y-2">
              {items.map((item) => {
                const href = withLocalePath(locale, item.href);
                const matchPaths =
                  item.matchPaths && item.matchPaths.length > 0
                    ? item.matchPaths
                    : [item.href];
                const isActive = matchPaths.some((path) => {
                  const localizedPath = withLocalePath(locale, path);
                  return (
                    pathname === localizedPath ||
                    pathname.startsWith(`${localizedPath}/`)
                  );
                });
                const Icon = item.icon;
                const navItem =
                  (
                    dictionary.navigation as Record<
                      string,
                      { label?: string; description?: string } | undefined
                    >
                  )[item.key] ?? {};

                return (
                  <SidebarNavItem
                    key={item.href}
                    href={href}
                    label={navItem.label ?? item.fallbackLabel}
                    Icon={Icon}
                    isActive={isActive}
                    collapsed={collapsed}
                    onClick={onCloseMobile}
                  />
                );
              })}
            </nav>
          </div>

          <div className="mt-4 shrink-0">
            <div
              className="mx-auto overflow-visible"
              style={{
                width: collapsed
                  ? `${BOTTOM_CONTROL_SIZE}px`
                  : `${BOTTOM_SWITCH_WIDTH}px`,
              }}
            >
              <DashboardSidebarLanguageSwitch
                locale={locale}
                collapsed={collapsed}
                onClick={onCloseMobile}
              />

              <div className="mt-4 hidden justify-center lg:flex">
                <button
                  type="button"
                  aria-label={
                    collapsed
                      ? dictionary.navigation.expandSidebar
                      : dictionary.navigation.collapseSidebar
                  }
                  onClick={onToggle}
                  className="inline-flex h-10 w-10 items-center justify-center rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface-elevated)] text-[color:var(--muted-foreground)] shadow-[0_8px_24px_rgba(0,0,0,0.18)] transition-colors hover:text-[color:var(--foreground)]"
                >
                  {collapsed ? (
                    <ChevronRightIcon className="h-5 w-5 stroke-[2.4]" />
                  ) : (
                    <ChevronLeftIcon className="h-5 w-5 stroke-[2.4]" />
                  )}
                </button>
              </div>
            </div>
          </div>

          <button
            type="button"
            onClick={onCloseMobile}
            className="mt-4 inline-flex h-11 w-full items-center justify-center rounded-2xl border border-[color:var(--button-secondary-border)] bg-[color:var(--button-secondary-bg)] px-4 text-sm font-medium text-[color:var(--button-secondary-fg)] transition-colors hover:bg-[color:var(--button-secondary-bg-hover)] lg:hidden"
          >
            {dictionary.common.close}
          </button>
        </div>
      </motion.aside>
    </>
  );
}

function SidebarNavItem({
  href,
  label,
  Icon,
  isActive,
  collapsed,
  onClick,
}: {
  href: string;
  label: string;
  Icon?: ComponentType<{ className?: string }>;
  isActive: boolean;
  collapsed: boolean;
  onClick?: () => void;
}) {
  const content = (
    <Link
      href={href}
      onClick={onClick}
      aria-label={collapsed ? label : undefined}
      className={cn(
        "block rounded-lg text-white transition-colors duration-200",
        isActive
          ? "bg-[color:var(--surface-elevated)]"
          : "hover:bg-[color:var(--button-ghost-bg-hover)]"
      )}
    >
      <div className="grid h-10 grid-cols-[44px_minmax(0,1fr)] items-center pl-[6px]">
        <div className="flex h-10 w-11 items-center justify-center">
          {Icon ? (
            <Icon className="h-5 w-5 shrink-0 text-[color:var(--foreground)] transition-colors duration-200" />
          ) : null}
        </div>

        <div className="min-w-0 overflow-hidden">
          <div
            className={cn(
              "overflow-hidden whitespace-nowrap transition-[max-width,opacity,margin-left] duration-300 ease-[cubic-bezier(0.22,1,0.36,1)]",
              collapsed
                ? "ml-0 max-w-0 opacity-0"
                : "ml-3 max-w-[180px] opacity-100"
            )}
          >
            <span className="block truncate text-[15px] font-medium leading-5 text-white">
              {label}
            </span>
          </div>
        </div>
      </div>
    </Link>
  );

  if (!collapsed) {
    return content;
  }

  return (
    <Tooltip.Provider delayDuration={100}>
      <Tooltip.Root>
        <Tooltip.Trigger asChild>{content}</Tooltip.Trigger>

        <Tooltip.Portal>
          <Tooltip.Content
            side="right"
            align="center"
            sideOffset={12}
            collisionPadding={8}
            className="z-[9999] whitespace-nowrap rounded-md bg-[color:var(--surface-elevated)] px-3 py-2 text-sm font-medium leading-none text-white shadow-[0_12px_30px_rgba(0,0,0,0.35)]"
          >
            {label}
            <Tooltip.Arrow className="fill-[color:var(--surface-elevated)]" />
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  );
}

export function DashboardPageContainer({ children }: { children: ReactNode }) {
  return <ConsolePage>{children}</ConsolePage>;
}

export function PageHeader({
  title,
  description,
  action,
  breadcrumbs,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
  breadcrumbs?: Array<{ label: string; href?: string }>;
}) {
  return (
    <header className="space-y-4">
      {breadcrumbs && breadcrumbs.length > 0 ? (
        <nav aria-label="Breadcrumb">
          <ol className="flex flex-wrap items-center gap-2 text-xs uppercase tracking-[0.16em] text-[color:var(--muted-foreground)]">
            {breadcrumbs.map((item, index) => (
              <li
                key={`${item.label}-${index}`}
                className="flex items-center gap-2"
              >
                {item.href ? (
                  <Link
                    href={item.href}
                    className="transition-colors hover:text-[color:var(--foreground)]"
                  >
                    {item.label}
                  </Link>
                ) : (
                  <span className="text-[color:var(--foreground)]">
                    {item.label}
                  </span>
                )}

                {index < breadcrumbs.length - 1 ? (
                  <span className="text-[color:var(--border-strong)]">/</span>
                ) : null}
              </li>
            ))}
          </ol>
        </nav>
      ) : null}

      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="max-w-4xl space-y-2">
          <h1 className="text-[28px] font-semibold tracking-tight text-[color:var(--foreground)]">
            {title}
          </h1>

          {description ? (
            <p className="text-sm leading-6 text-[color:var(--muted-foreground)]">
              {description}
            </p>
          ) : null}
        </div>

        {action ? (
          <div className="flex flex-wrap items-center gap-2 lg:justify-end">
            {action}
          </div>
        ) : null}
      </div>
    </header>
  );
}

export function Section({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={cn(
        "min-w-0 border-t border-[color:var(--border)] py-5",
        className
      )}
    >
      {children}
    </section>
  );
}

function getSidebarIcon(item: Pick<NavItem, "key">) {
  switch (item.key) {
    case "overview":
      return HomeIcon;
    case "infrastructure":
      return ServerIcon;
    case "security":
      return ShieldIcon;
    case "operations":
      return TerminalIcon;
    case "integrations":
      return GridIcon;
    case "audit":
      return ActivityIcon;
    case "profile":
      return UserIcon;
    default:
      return undefined;
  }
}

function getSidebarUsername(
  user:
    | {
        username?: string | null;
        login?: string | null;
        displayName?: string | null;
        email?: string | null;
      }
    | null
    | undefined
) {
  const rawValue =
    user?.username ??
    user?.login ??
    user?.displayName ??
    user?.email?.split("@")[0] ??
    "admin";

  return rawValue.trim().replace(/^@+/, "").replace(/\s+/g, "").toLowerCase();
}
