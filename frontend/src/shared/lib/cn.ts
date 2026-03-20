export type ClassValue =
  | string
  | number
  | null
  | undefined
  | boolean
  | ClassValue[]
  | Record<string, boolean | null | undefined>;

function clsx(...inputs: ClassValue[]): string {
  const result: string[] = [];

  for (const input of inputs) {
    if (!input) continue;

    if (typeof input === "string" || typeof input === "number") {
      result.push(String(input));
      continue;
    }

    if (Array.isArray(input)) {
      const nested = clsx(...input);
      if (nested) result.push(nested);
      continue;
    }

    if (typeof input === "object") {
      for (const key in input) {
        if (input[key]) result.push(key);
      }
    }
  }

  return result.join(" ");
}

function twMerge(className: string): string {
  return className.replace(/\s+/g, " ").trim();
}

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(...inputs));
}
