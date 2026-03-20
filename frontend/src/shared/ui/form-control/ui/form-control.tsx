import { ReactNode } from "react";
import { cn } from "@/src/shared/lib/cn";

type FormControlProps = {
  children: ReactNode;
  hasError?: boolean;
  className?: string;
};

export function FormControl({
  children,
  hasError = false,
  className,
}: FormControlProps) {
  return (
    <div
      className={cn(className, hasError ? "[&>*]:border-red-500" : undefined)}
    >
      {children}
    </div>
  );
}

