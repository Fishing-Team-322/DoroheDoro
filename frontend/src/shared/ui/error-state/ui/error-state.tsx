import { ReactNode } from "react";
import { cn } from "@/src/shared/lib/cn";

interface ErrorStateProps {
  title?: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function ErrorState({
  title = "Что-то пошло не так",
  description,
  action,
  className,
}: ErrorStateProps) {
  return (
    <div
      className={cn(
        "rounded-lg border border-[color:var(--danger-border-soft)] bg-[color:var(--danger-bg-soft)] p-6 text-center",
        className
      )}
    >
      <h3 className="text-base font-semibold text-[color:var(--danger-fg)]">{title}</h3>
      {description ? (
        <p className="mt-1 text-sm text-[color:var(--danger-fg)]">{description}</p>
      ) : null}
      {action ? <div className="mt-4">{action}</div> : null}
    </div>
  );
}

