"use client";

import { ReactNode } from "react";
import { cn } from "@/src/shared/lib/cn";
import { Button } from "@/src/shared/ui/button";

interface DialogProps {
  open: boolean;
  title?: string;
  description?: string;
  onClose: () => void;
  children?: ReactNode;
  className?: string;
}

export function Dialog({
  open,
  title,
  description,
  onClose,
  children,
  className,
}: DialogProps) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div
        className={cn(
          "w-full max-w-md rounded-xl border border-zinc-800 bg-zinc-950 p-5 text-[color:var(--card-foreground)] shadow-xl shadow-black/30",
          className
        )}
      >
        <div className="mb-4 space-y-1">
          {title ? <h3 className="text-lg font-semibold">{title}</h3> : null}
          {description ? (
            <p className="text-sm text-[color:var(--muted-foreground)]">
              {description}
            </p>
          ) : null}
        </div>
        <div>{children}</div>
        <div className="mt-5 flex justify-end">
          <Button variant="secondary" onClick={onClose}>
            Close
          </Button>
        </div>
      </div>
    </div>
  );
}
