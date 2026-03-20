"use client";

import type { ComponentType, ReactNode } from "react";
import { useMemo, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/src/shared/lib/cn";
import { ConsolePage } from "@/src/shared/ui";
import { withLocalePath } from "@/src/shared/lib/i18n";
import type { Locale } from "@/src/shared/config";
import { useAuth } from "@/src/features/auth/model/use-auth";
import { LogoutButton } from "@/src/features/auth/ui/logout-button";
import { dashboardNavigation } from "@/src/features/dashboard-navigation";
import {
  ActivityIcon,
  BellIcon,
  ChartIcon,
  ChevronLeftIcon,
  ChevronRightIcon,
  GridIcon,
  HomeIcon,
  LogoGlyph,
  LogsIcon,
  PulseIcon,
  RocketIcon,
  ServerIcon,
  SettingsIcon,
  ShieldIcon,
} from "../icons";

type DashboardLayoutProps = {
  locale: Locale;
  children: ReactNode;
};

type NavItem = {
  href: string;
  label: string;
  description?: string;
  icon?: ComponentType<{ className?: string }>;
};

export function DashboardLayout({ locale, children }: DashboardLayoutProps) {
  const [collapsed, setCollapsed] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <div className="min-h-screen bg-[color:var(--background)] text-[color:var(--foreground)]">
      <div className="flex min-h-screen">
        <DashboardSidebar
          locale={locale}
          collapsed={collapsed}
          mobileOpen={mobileOpen}
          onToggle={() => setCollapsed((current) => !current)}
          onCloseMobile={() => setMobileOpen(false)}
        />

        <main className="relative min-h-screen min-w-0 flex-1 overflow-y-auto">
          <DashboardPageContainer>{children}</DashboardPageContainer>
        </main>
      </div>
    </div>
  );
}

type DashboardSidebarProps = {
  locale: Locale;
  collapsed: boolean;
  mobileOpen: boolean;
  onToggle: () => void;
  onCloseMobile: () => void;
};

export function DashboardSidebar({
  locale,
  collapsed,
  mobileOpen,
  onToggle,
  onCloseMobile,
}: DashboardSidebarProps) {
  const pathname = usePathname();
  const { user } = useAuth();

  const items = useMemo<NavItem[]>(
    () =>
      dashboardNavigation.map((item) => ({
        ...item,
        icon: getSidebarIcon(item),
      })),
    []
  );

  return (
    <>
      <div
        className={cn(
          "fixed inset-0 z-30 bg-black/50 backdrop-blur-[2px] transition-opacity duration-300 lg:hidden",
          mobileOpen ? "pointer-events-auto opacity-100" : "pointer-events-none opacity-0"
        )}
        onClick={onCloseMobile}
      />

      <aside
        className={cn(
          "group/sidebar relative fixed inset-y-0 left-0 z-40 flex h-screen shrink-0 flex-col border-r border-[color:var(--border)] bg-[color:var(--background)] transition-all duration-300 ease-out lg:sticky lg:translate-x-0",
          mobileOpen ? "translate-x-0" : "-translate-x-full",
          collapsed ? "w-[92px]" : "w-[296px]"
        )}
      >
        <button
          type="button"
          aria-label={collapsed ? "Развернуть боковую панель" : "Свернуть боковую панель"}
          onClick={onToggle}
          className={cn(
            "absolute right-2 top-5 z-20 hidden items-center justify-center rounded-xl bg-[color:var(--surface-elevated)] text-[color:var(--muted-foreground)] opacity-0 shadow-sm pointer-events-none transition-all duration-200 hover:bg-[color:var(--surface-elevated)] hover:text-[color:var(--foreground)] group-hover/sidebar:pointer-events-auto group-hover/sidebar:opacity-100 lg:inline-flex",
            collapsed ? "h-10 w-10" : "h-12 w-9"
          )}
        >
          {collapsed ? (
            <ChevronRightIcon className="h-6 w-6 stroke-[2.6]" />
          ) : (
            <ChevronLeftIcon className="h-6 w-6 stroke-[2.6]" />
          )}
        </button>

        <div className="flex h-full min-h-0 flex-col px-4 pb-4 pt-5">
          <div
            className={cn(
              "mb-6 flex items-start gap-3",
              collapsed ? "justify-center pr-0" : "pr-10"
            )}
          >
            <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl bg-[color:var(--surface-elevated)]">
              <LogoGlyph />
            </div>

            <div className={cn("min-w-0", collapsed && "hidden")}>
              <p className="truncate text-[11px] uppercase tracking-[0.22em] text-[color:var(--muted-foreground)]">
                Панель управления
              </p>
              <p className="truncate text-[15px] font-semibold leading-none text-[color:var(--foreground)]">
                Консоль инфраструктуры
              </p>
            </div>
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto pr-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            <nav className="space-y-1">
              {items.map((item) => {
                const href = withLocalePath(locale, item.href);
                const isActive = pathname === href || pathname.startsWith(`${href}/`);
                const Icon = item.icon;

                return (
                  <Link
                    key={item.href}
                    href={href}
                    onClick={onCloseMobile}
                    title={collapsed ? item.label : undefined}
                    className={cn(
                      "group/item flex h-11 items-center gap-3 rounded-xl px-3 py-2 text-sm transition-colors duration-150",
                      collapsed && "justify-center px-0",
                      isActive
                        ? "bg-[color:var(--surface-elevated)] text-[color:var(--foreground)]"
                        : "text-[color:var(--muted-foreground)] hover:bg-[color:var(--button-ghost-bg-hover)] hover:text-[color:var(--foreground)]"
                    )}
                  >
                    {Icon ? (
                      <Icon
                        className={cn(
                          "h-5 w-5 shrink-0 transition-colors",
                          isActive
                            ? "text-[color:var(--foreground)]"
                            : "text-[color:var(--muted-foreground)] group-hover/item:text-[color:var(--foreground)]"
                        )}
                      />
                    ) : null}

                    <span className={cn("min-w-0 flex-1", collapsed && "hidden")}>
                      <span className="block truncate text-[15px] font-medium leading-5">
                        {item.label}
                      </span>
                    </span>
                  </Link>
                );
              })}
            </nav>
          </div>

          <div
            className={cn(
              "mt-4 rounded-2xl border border-[color:var(--border)] bg-[color:var(--surface-elevated)] p-3",
              collapsed && "px-2 py-2"
            )}
          >
            <div className={cn("space-y-1", collapsed && "hidden")}>
              <p className="truncate text-sm font-medium text-[color:var(--foreground)]">
                {user?.displayName ?? "Signed in"}
              </p>
              <p className="truncate text-xs text-[color:var(--muted-foreground)]">
                {user?.email ?? "Session active"}
              </p>
            </div>

            <LogoutButton
              locale={locale}
              variant={collapsed ? "ghost" : "secondary"}
              size="sm"
              className={cn("mt-3 w-full", collapsed && "mt-0 px-0")}
            >
              {collapsed ? "Out" : "Sign out"}
            </LogoutButton>
          </div>

          <button
            type="button"
            onClick={onCloseMobile}
            className="mt-4 inline-flex h-10 w-full items-center justify-center rounded-2xl border border-[color:var(--button-secondary-border)] bg-[color:var(--button-secondary-bg)] px-4 text-sm font-medium text-[color:var(--button-secondary-fg)] transition-colors hover:bg-[color:var(--button-secondary-bg-hover)] lg:hidden"
          >
            Закрыть
          </button>
        </div>
      </aside>
    </>
  );
}

export function DashboardPageContainer({
  children,
}: {
  children: ReactNode;
}) {
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
              <li key={`${item.label}-${index}`} className="flex items-center gap-2">
                {item.href ? (
                  <Link
                    href={item.href}
                    className="transition-colors hover:text-[color:var(--foreground)]"
                  >
                    {item.label}
                  </Link>
                ) : (
                  <span className="text-[color:var(--foreground)]">{item.label}</span>
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
          <div className="flex flex-wrap items-center gap-2 lg:justify-end">{action}</div>
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
    <section className={cn("min-w-0 border-t border-[color:var(--border)] py-5", className)}>
      {children}
    </section>
  );
}

function getSidebarIcon(item: Pick<NavItem, "href" | "label">) {
  const key = `${item.href} ${item.label}`.toLowerCase();

  if (key.includes("overview") || key.includes("обзор") || key.includes("home")) {
    return HomeIcon;
  }
  if (key.includes("inventory") || key.includes("host")) {
    return ServerIcon;
  }
  if (key.includes("polic")) {
    return ShieldIcon;
  }
  if (key.includes("deploy")) {
    return RocketIcon;
  }
  if (key.includes("logs") || key.includes("журнал")) {
    return LogsIcon;
  }
  if (key.includes("live") || key.includes("поток")) {
    return ActivityIcon;
  }
  if (key.includes("dashboard") || key.includes("панел")) {
    return ChartIcon;
  }
  if (key.includes("alert") || key.includes("оповещ")) {
    return BellIcon;
  }
  if (key.includes("diagnostic") || key.includes("диагност")) {
    return PulseIcon;
  }
  if (key.includes("setting") || key.includes("настро")) {
    return SettingsIcon;
  }

  if (key.includes("profile") || key.includes("account")) {
    return SettingsIcon;
  }

  return GridIcon;
}
