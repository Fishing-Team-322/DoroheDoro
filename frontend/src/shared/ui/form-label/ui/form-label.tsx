import { LabelHTMLAttributes } from "react";

type FormLabelProps = LabelHTMLAttributes<HTMLLabelElement>;

export function FormLabel({ className, ...props }: FormLabelProps) {
  return (
    <label
      className={`text-sm font-medium text-[color:var(--foreground)] ${className ?? ""}`.trim()}
      {...props}
    />
  );
}
