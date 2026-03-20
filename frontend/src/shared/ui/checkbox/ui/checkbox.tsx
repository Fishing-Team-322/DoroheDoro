"use client";

import { InputHTMLAttributes } from "react";
import { cn } from "@/src/shared/lib/cn";

export interface CheckboxProps extends Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "type"
> {
  label?: string;
}

export function Checkbox({
  className,
  label,
  disabled,
  ...props
}: CheckboxProps) {
  return (
    <label
      className={cn(
        "inline-flex items-center gap-2 text-sm",
        disabled && "opacity-50",
        className
      )}
    >
      <input
        type="checkbox"
        className="h-4 w-4 rounded border-[color:var(--input-border)] bg-[color:var(--input-background)]"
        disabled={disabled}
        {...props}
      />
      {label ? <span className="text-[color:var(--foreground)]">{label}</span> : null}
    </label>
  );
}

