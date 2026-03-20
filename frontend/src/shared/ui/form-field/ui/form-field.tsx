import { ReactNode } from "react";

type FormFieldProps = {
  children: ReactNode;
  className?: string;
};

export function FormField({ children, className }: FormFieldProps) {
  return (
    <div className={`flex flex-col gap-2 ${className ?? ""}`.trim()}>
      {children}
    </div>
  );
}
