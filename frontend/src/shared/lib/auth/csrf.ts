const CSRF_COOKIE_NAME = "csrf_token";

let csrfTokenCache: string | null = null;

function decodeCookieValue(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

export function readCookie(name: string): string | null {
  if (typeof document === "undefined") {
    return null;
  }

  const prefix = `${name}=`;
  const match = document.cookie
    .split("; ")
    .find((cookiePart) => cookiePart.startsWith(prefix));

  if (!match) {
    return null;
  }

  return decodeCookieValue(match.slice(prefix.length));
}

export function getCsrfToken(): string | null {
  return csrfTokenCache ?? readCookie(CSRF_COOKIE_NAME);
}

export function setCsrfToken(token?: string | null): void {
  csrfTokenCache = token?.trim() ? token : null;
}

export function clearCsrfToken(): void {
  csrfTokenCache = null;
}

export { CSRF_COOKIE_NAME };

export async function fetchCsrfToken(baseUrl?: string): Promise<string> {
  const response = await fetch(`${baseUrl ?? ""}/auth/csrf`, {
    credentials: "include",
    method: "GET",
  });

  if (!response.ok) {
    throw new Error(`Unable to initialize CSRF token (${response.status})`);
  }

  const payload = (await response.json()) as { csrfToken?: string };
  const token = payload.csrfToken?.trim();

  if (!token) {
    throw new Error("Edge API did not return a CSRF token");
  }

  setCsrfToken(token);
  return token;
}
