"use client";

import Link from "next/link";
import { cn } from "@/src/shared/lib/cn";

export type SectionTabItem = {
  id: string;
  label: string;
  href: string;
};

export function SectionTabs({
  items,
  activeId,
  ariaLabel,
}: {
  items: SectionTabItem[];
  activeId: string;
  ariaLabel: string;
}) {
  return (
    <nav
      role="tablist"
      aria-label={ariaLabel}
      className="flex flex-wrap gap-2 rounded-xl border border-[color:var(--border)] bg-[color:var(--surface)] p-2"
    >
      {items.map((item) => {
        const active = item.id === activeId;

        return (
          <Link
            key={item.id}
            href={item.href}
            aria-current={active ? "page" : undefined}
            className={cn(
              "rounded-lg px-4 py-2.5 text-sm font-medium transition-colors",
              active
                ? "bg-[color:var(--surface-elevated)] text-[color:var(--foreground)]"
                : "text-[color:var(--muted-foreground)] hover:bg-[color:var(--surface-elevated)] hover:text-[color:var(--foreground)]"
            )}
          >
            {item.label}
          </Link>
        );
      })}
    </nav>
  );
}
