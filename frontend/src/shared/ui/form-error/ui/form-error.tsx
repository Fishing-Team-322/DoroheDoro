type FormErrorProps = {
  message?: string;
  className?: string;
};

export function FormError({ message, className }: FormErrorProps) {
  if (!message) {
    return null;
  }

  return (
    <p
      role="alert"
      className={`text-sm text-red-600 ${className ?? ""}`.trim()}
    >
      {message}
    </p>
  );
}
