import { LabelHTMLAttributes } from "react";
import { cn } from "@/src/shared/lib/cn";

type LabelProps = LabelHTMLAttributes<HTMLLabelElement>;

export function Label({ className, ...props }: LabelProps) {
  return (
    <label
      className={cn(
        "text-sm font-medium leading-none text-[color:var(--foreground)]",
        className
      )}
      {...props}
    />
  );
}

