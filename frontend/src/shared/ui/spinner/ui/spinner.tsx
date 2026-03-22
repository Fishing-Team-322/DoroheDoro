import { HTMLAttributes } from "react";
import { cn } from "@/src/shared/lib/cn";
import "./spinner.css";

interface SpinnerProps extends HTMLAttributes<HTMLDivElement> {
  size?: "sm" | "md" | "lg";
  center?: boolean;
}

const sizeClasses: Record<NonNullable<SpinnerProps["size"]>, string> = {
  sm: "spinner-sm",
  md: "spinner-md",
  lg: "spinner-lg",
};

export function Spinner({
  className,
  size = "md",
  center = false,
  ...props
}: SpinnerProps) {
  return (
    <div
      className={cn(
        "spinner",
        sizeClasses[size],
        center && "center",
        className
      )}
      aria-label="Loading"
      role="status"
      {...props}
    >
      {Array.from({ length: 12 }).map((_, index) => (
        <div key={index} className="spinner-blade" />
      ))}
    </div>
  );
}