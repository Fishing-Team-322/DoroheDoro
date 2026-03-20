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
