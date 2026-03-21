"use client";

import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "@/src/shared/lib/cn";

type ConsolePageProps = HTMLAttributes<HTMLElement> & {
  children: ReactNode;
};

export function ConsolePage({
  children,
  className,
  ...props
}: ConsolePageProps) {
  return (
    <div
      className={cn("flex w-full min-w-0 flex-col overflow-x-clip", className)}
      {...props}
    >
      {children}
    </div>
  );
}
