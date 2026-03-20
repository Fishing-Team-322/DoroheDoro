import { HTMLAttributes } from "react";
import { cn } from "@/src/shared/lib/cn";

type CardProps = HTMLAttributes<HTMLDivElement>;

export function Card({ className, ...props }: CardProps) {
  return (
    <div
      className={cn(
        "rounded-xl border border-[color:var(--border)] bg-[color:var(--surface-elevated)] p-4 text-[color:var(--card-foreground)] shadow-sm shadow-black/20",
        className
      )}
      {...props}
    />
  );
}

