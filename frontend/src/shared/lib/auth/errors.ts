import type { ApiError } from "@/src/shared/lib/api";
import { isApiError } from "@/src/shared/lib/api";

export type LoginErrorStage = "csrf_init" | "login";
export type LoginErrorKind =
  | "invalid_credentials"
  | "csrf"
  | "proxy"
  | "network"
  | "server"
  | "not_available"
  | "unknown";

export class LoginFlowError extends Error {
  readonly name = "LoginFlowError";

  constructor(
    readonly kind: LoginErrorKind,
    readonly stage: LoginErrorStage,
    message: string,
    readonly status: number | null,
    readonly cause?: unknown
  ) {
    super(message);
  }
}

const PROXY_STATUS_CODES = new Set([502, 503, 504]);

function readResponseMessage(error: ApiError): string {
  return typeof error.message === "string" ? error.message.toLowerCase() : "";
}

function hasCsrfSignal(error: ApiError): boolean {
  if (/csrf/.test(readResponseMessage(error))) {
    return true;
  }

  const details = error.details;
  return (
    typeof details === "object" &&
    details !== null &&
    typeof (details as { message?: unknown }).message === "string" &&
    /csrf/.test(
      ((details as { message: string }).message ?? "").toLowerCase()
    )
  );
}

function messageFor(kind: LoginErrorKind, stage: LoginErrorStage): string {
  if (kind === "invalid_credentials") {
    return "Неверный логин, email или пароль.";
  }

  if (kind === "csrf" && stage === "csrf_init") {
    return "Не удалось инициализировать CSRF-защиту формы входа. Обновите страницу и попробуйте снова.";
  }

  if (kind === "csrf") {
    return "CSRF-проверка не прошла. Обновите страницу и попробуйте войти снова.";
  }

  if ((kind === "proxy" || kind === "network") && stage === "csrf_init") {
    return "Не удалось получить CSRF-токен от сервера авторизации. Проверьте backend/proxy и обновите страницу.";
  }

  if (kind === "proxy" || kind === "network") {
    return "Не удалось подключиться к серверу авторизации. Проверьте backend/proxy и попробуйте снова.";
  }

  if (kind === "not_available") {
    return "Сервис авторизации не настроен в текущем окружении.";
  }

  if (kind === "server") {
    return "Сервис авторизации временно недоступен. Попробуйте позже.";
  }

  if (stage === "csrf_init") {
    return "Не удалось подготовить защищённый вход. Обновите страницу и попробуйте снова.";
  }

  return "Не удалось выполнить вход из-за неожиданной ошибки.";
}

function classifyApiError(
  error: ApiError,
  stage: LoginErrorStage
): LoginErrorKind {
  if (stage === "login" && error.status === 401) {
    return "invalid_credentials";
  }

  if (error.code === "proxy_upstream_unavailable") {
    return "proxy";
  }

  if (hasCsrfSignal(error) || error.status === 403) {
    return "csrf";
  }

  if (error.code === "NETWORK_ERROR" || error.status === null) {
    return "network";
  }

  if (error.status !== null && PROXY_STATUS_CODES.has(error.status)) {
    return "proxy";
  }

  if (error.status === 501) {
    return "not_available";
  }

  if (error.status !== null && error.status >= 500) {
    return "server";
  }

  return "unknown";
}

export function isLoginFlowError(error: unknown): error is LoginFlowError {
  return error instanceof Error && error.name === "LoginFlowError";
}

export function normalizeLoginError(
  error: unknown,
  stage: LoginErrorStage
): LoginFlowError {
  if (isLoginFlowError(error)) {
    return error;
  }

  if (isApiError(error)) {
    const kind = classifyApiError(error, stage);
    return new LoginFlowError(
      kind,
      stage,
      messageFor(kind, stage),
      error.status,
      error
    );
  }

  if (error instanceof Error) {
    return new LoginFlowError(
      "unknown",
      stage,
      messageFor("unknown", stage),
      null,
      error
    );
  }

  return new LoginFlowError(
    "unknown",
    stage,
    messageFor("unknown", stage),
    null,
    error
  );
}

export function getLoginErrorMessage(error: unknown): string {
  return normalizeLoginError(error, "login").message;
}
