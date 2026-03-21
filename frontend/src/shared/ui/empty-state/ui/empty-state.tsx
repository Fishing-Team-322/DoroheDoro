import { ReactNode } from "react";
import { cn } from "@/src/shared/lib/cn";

interface EmptyStateProps {
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
  variant?: "flush" | "inset";
}

export function EmptyState({
  title,
  description,
  action,
  className,
  variant = "inset",
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        variant === "inset" &&
          "rounded-lg border border-dashed border-[color:var(--border)] bg-[color:var(--surface-subtle)] p-6 text-center",
        variant === "flush" && "py-8 text-center",
        className
      )}
    >
      <h3 className="text-lg font-semibold text-[color:var(--foreground)]">{title}</h3>
      {description ? (
        <p className="mt-1 text-base text-[color:var(--muted-foreground)]">
          {description}
        </p>
      ) : null}
      {action ? <div className="mt-4">{action}</div> : null}
    </div>
  );
}

