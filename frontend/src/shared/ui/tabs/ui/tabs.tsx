"use client";

import { ReactNode, useState } from "react";
import { cn } from "@/src/shared/lib/cn";

export interface TabItem {
  key: string;
  label: string;
  content: ReactNode;
  disabled?: boolean;
}

interface TabsProps {
  tabs: TabItem[];
  defaultValue?: string;
  className?: string;
}

export function Tabs({ tabs, defaultValue, className }: TabsProps) {
  const [activeTab, setActiveTab] = useState(defaultValue ?? tabs[0]?.key);
  const current = tabs.find((tab) => tab.key === activeTab);

  return (
    <div className={cn("space-y-3", className)}>
      <div className="inline-flex rounded-lg bg-zinc-900 p-1">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            type="button"
            disabled={tab.disabled}
            onClick={() => setActiveTab(tab.key)}
            className={cn(
              "rounded-md px-3 py-1.5 text-sm transition-colors disabled:opacity-50",
              activeTab === tab.key
                ? "bg-zinc-50 text-zinc-950 shadow-sm"
                : "text-[color:var(--muted-foreground)] hover:text-zinc-100"
            )}
          >
            {tab.label}
          </button>
        ))}
      </div>
      <div>{current?.content}</div>
    </div>
  );
}

