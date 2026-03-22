import { HTMLAttributes } from "react";
import { cn } from "@/src/shared/lib/cn";

type CardProps = HTMLAttributes<HTMLDivElement>;

export function Card({ className, ...props }: CardProps) {
  return (
    <div
      className={cn(
        "rounded-[28px] border border-[color:var(--border)] bg-[color:var(--surface)] p-8 md:p-10",
        className
      )}
      {...props}
    />
  );
}

