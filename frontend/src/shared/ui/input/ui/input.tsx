"use client";

import { forwardRef, InputHTMLAttributes } from "react";
import { cn } from "@/src/shared/lib/cn";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  inputSize?: "sm" | "md" | "lg";
}

const sizeClasses: Record<NonNullable<InputProps["inputSize"]>, string> = {
  sm: "h-8 px-3 text-sm",
  md: "h-10 px-3 text-sm",
  lg: "h-11 px-4 text-base",
};

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { className, inputSize = "md", ...props },
  ref
) {
  return (
    <input
      ref={ref}
      className={cn(
        "flex w-full rounded-md border border-[color:var(--input-border)] bg-[color:var(--input-background)] text-[color:var(--foreground)] ring-offset-[color:var(--input-background)] file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-[color:var(--muted-foreground)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[color:var(--ring)] disabled:cursor-not-allowed disabled:opacity-50",
        sizeClasses[inputSize],
        className
      )}
      {...props}
    />
  );
});

