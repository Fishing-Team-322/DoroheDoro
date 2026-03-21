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
      className={cn(
        "flex w-full min-w-0 flex-col overflow-x-clip px-4 pt-4 pb-6 sm:px-6 sm:pt-6 sm:pb-8 lg:px-8 lg:pt-8 lg:pb-10",
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
}