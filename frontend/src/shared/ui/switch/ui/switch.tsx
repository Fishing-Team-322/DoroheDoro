"use client";

import { forwardRef, InputHTMLAttributes, useId } from "react";
import { cn } from "@/src/shared/lib/cn";

export interface SwitchProps
  extends Omit<InputHTMLAttributes<HTMLInputElement>, "type"> {
  switchLabel?: string;
  onCheckedChange?: (checked: boolean) => void;
}

export const Switch = forwardRef<HTMLInputElement, SwitchProps>(function Switch(
  {
    className,
    id,
    disabled,
    switchLabel,
    onChange,
    onCheckedChange,
    ...props
  },
  ref
) {
  const generatedId = useId();
  const switchId = id ?? generatedId;

  return (
    <label
      htmlFor={switchId}
      className={cn(
        "inline-flex select-none items-center gap-3",
        disabled ? "cursor-not-allowed opacity-70" : "cursor-pointer",
        className
      )}
    >
      <span className="relative inline-block h-10 w-20 shrink-0">
        <input
          {...props}
          ref={ref}
          id={switchId}
          type="checkbox"
          role="switch"
          disabled={disabled}
          onChange={(event) => {
            onChange?.(event);
            onCheckedChange?.(event.target.checked);
          }}
          className="peer sr-only"
        />

        <span
          aria-hidden="true"
          className={cn(
            "absolute inset-0 rounded-[20px] bg-[#ddd] shadow-[inset_0_0_0_2px_#ccc]",
            "transition-[background-color,box-shadow] duration-300 ease-in-out",
            "after:absolute after:left-[5px] after:top-[5px] after:h-[30px] after:w-[30px]",
            "after:rounded-full after:bg-white after:shadow-[0_2px_5px_rgba(0,0,0,0.2)]",
            "after:transition-[transform,box-shadow] after:duration-300 after:ease-in-out",
            "peer-checked:bg-[#05c46b] peer-checked:shadow-[inset_0_0_0_2px_#04b360]",
            "peer-checked:after:translate-x-[40px]",
            "peer-checked:after:shadow-[0_2px_5px_rgba(0,0,0,0.2),0_0_0_3px_#05c46b]",
            "peer-focus-visible:ring-2 peer-focus-visible:ring-[color:var(--ring)]",
            "peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-[color:var(--background)]",
            "peer-disabled:opacity-60"
          )}
        />
      </span>

      {switchLabel ? (
        <span className="text-sm font-medium text-[color:var(--foreground)]">
          {switchLabel}
        </span>
      ) : null}
    </label>
  );
});
