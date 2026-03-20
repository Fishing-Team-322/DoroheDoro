"use client";

import { InputHTMLAttributes } from "react";
import { cn } from "@/src/shared/lib/cn";

interface SearchInputProps extends Omit<
  InputHTMLAttributes<HTMLInputElement>,
  "type"
> {
  loading?: boolean;
}

export function SearchInput({
  className,
  loading = false,
  disabled,
  ...props
}: SearchInputProps) {
  return (
    <div className="relative">
      <input
        type="search"
        className={cn(
          "w-full rounded-md border border-[color:var(--input-border)] bg-[color:var(--input-background)] py-2 pl-9 pr-10 text-sm text-[color:var(--foreground)] placeholder:text-[color:var(--muted-foreground)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--ring)] disabled:cursor-not-allowed disabled:opacity-50",
          className
        )}
        disabled={disabled || loading}
        {...props}
      />
      <span className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-[color:var(--muted-foreground)]">
        ?
      </span>
      {loading ? (
        <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-[color:var(--muted-foreground)]">
          ...
        </span>
      ) : null}
    </div>
  );
}
