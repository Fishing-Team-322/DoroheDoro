"use client";

import { TextareaHTMLAttributes } from "react";
import { cn } from "@/src/shared/lib/cn";

export interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  textareaSize?: "sm" | "md" | "lg";
}

const sizeClasses: Record<
  NonNullable<TextareaProps["textareaSize"]>,
  string
> = {
  sm: "min-h-20 p-2 text-sm",
  md: "min-h-24 p-3 text-sm",
  lg: "min-h-32 p-4 text-base",
};

export function Textarea({
  className,
  textareaSize = "md",
  ...props
}: TextareaProps) {
  return (
    <textarea
      className={cn(
        "flex w-full rounded-md border border-[color:var(--input-border)] bg-[color:var(--input-background)] text-[color:var(--foreground)] ring-offset-[color:var(--input-background)] placeholder:text-[color:var(--muted-foreground)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--ring)] disabled:cursor-not-allowed disabled:opacity-50",
        sizeClasses[textareaSize],
        className
      )}
      {...props}
    />
  );
}

