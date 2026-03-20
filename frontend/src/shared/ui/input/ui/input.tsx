"use client";

import { forwardRef, InputHTMLAttributes, useId } from "react";
import { cn } from "@/src/shared/lib/cn";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  inputSize?: "sm" | "md" | "lg";
  label?: string;
  helperText?: string;
  error?: string;
  containerClassName?: string;
}

const sizeClasses: Record<NonNullable<InputProps["inputSize"]>, string> = {
  sm: "h-12 px-4 pt-6 pb-2 text-sm",
  md: "h-14 px-4 pt-6 pb-2 text-sm",
  lg: "h-14 px-4 pt-6 pb-2 text-base",
};

const labelPositionClasses: Record<
  NonNullable<InputProps["inputSize"]>,
  string
> = {
  sm: "left-4 top-4 text-sm",
  md: "left-4 top-4 text-sm",
  lg: "left-4 top-4 text-base",
};

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  {
    className,
    containerClassName,
    inputSize = "md",
    label,
    helperText,
    error,
    id,
    disabled,
    ...props
  },
  ref
) {
  const generatedId = useId();
  const inputId = id ?? generatedId;

  const describedBy = error
    ? `${inputId}-error`
    : helperText
      ? `${inputId}-helper`
      : undefined;

  return (
    <div className={cn("w-full", containerClassName)}>
      <div className="relative">
        <input
          ref={ref}
          id={inputId}
          disabled={disabled}
          aria-invalid={Boolean(error)}
          aria-describedby={describedBy}
          placeholder=" "
          className={cn(
            "peer w-full rounded-md border bg-[var(--input-background)] outline-none transition-all duration-200",
            "text-[var(--foreground)] placeholder:text-transparent",
            "disabled:cursor-not-allowed disabled:opacity-50",
            error
              ? "border-[var(--status-danger-border)] focus:border-[var(--status-danger-border)] focus:bg-[var(--input-background-focus)] focus:shadow-[0_0_0_1px_var(--status-danger-border),0_0_0_2px_rgba(153,27,27,0.08)]"
              : [
                  "border-[var(--input-border)]",
                  "hover:border-[var(--input-border-hover)]",
                  "hover:bg-[var(--input-background-hover)]",
                  "focus:border-[var(--ring)]",
                  "focus:bg-[var(--input-background-focus)]",
                  "focus:shadow-[0_0_0_1px_var(--ring),0_0_0_2px_rgba(113,113,122,0.08)]",
                ].join(" "),
            sizeClasses[inputSize],
            className
          )}
          {...props}
        />

        {label ? (
          <label
            htmlFor={inputId}
            className={cn(
              "pointer-events-none absolute z-10 origin-[0] bg-transparent px-0 transition-all duration-150",
              "text-[#8a8a8f]",
              labelPositionClasses[inputSize],
              "-translate-y-3 scale-90",
              "peer-placeholder-shown:translate-y-0 peer-placeholder-shown:scale-100",
              "peer-focus:-translate-y-3 peer-focus:scale-90",
              "peer-[&:not(:placeholder-shown)]:-translate-y-3",
              "peer-[&:not(:placeholder-shown)]:scale-90",
              error && "peer-focus:text-[var(--status-danger-fg)]"
            )}
          >
            {label}
          </label>
        ) : null}
      </div>

      {error ? (
        <p
          id={`${inputId}-error`}
          className="pt-2 text-sm text-[var(--status-danger-fg)]"
        >
          {error}
        </p>
      ) : helperText ? (
        <p
          id={`${inputId}-helper`}
          className="pt-2 text-sm text-[var(--muted-foreground)]"
        >
          {helperText}
        </p>
      ) : null}
    </div>
  );
});
