import { createApiClient } from "@/src/shared/lib/api";
import { ApiError } from "@/src/shared/lib/api/client";
import { clearCsrfToken, fetchCsrfToken, getCsrfToken, setCsrfToken } from "./csrf";
import { emitUnauthorized } from "./events";
import type {
  LoginInput,
  SessionPayload,
  UpdateProfileInput,
} from "./types";

const authApiClient = createApiClient({
  baseUrl: process.env.NEXT_PUBLIC_API_BASE_URL ?? "/api/edge",
  credentials: "include",
  getCsrfToken,
  onUnauthorized: () => {
    clearCsrfToken();
    emitUnauthorized();
  },
});


async function ensureCsrfToken(): Promise<string> {
  const existingToken = getCsrfToken();
  if (existingToken) {
    return existingToken;
  }

  return fetchCsrfToken(process.env.NEXT_PUBLIC_API_BASE_URL ?? "/api/edge");
}

export function isUnauthorizedError(error: unknown): boolean {
  return (
    error instanceof Error &&
    (error as ApiError).name === "ApiError" &&
    (error as ApiError).status === 401
  );
}

export async function login(input: LoginInput): Promise<SessionPayload> {
  await ensureCsrfToken();
  const response = await authApiClient.post<SessionPayload>("/auth/login", {
    identifier: input.identifier,
    email: input.identifier,
    login: input.identifier,
    password: input.password,
  });
  setCsrfToken(response.csrfToken);
  return response;
}

export async function logout(): Promise<void> {
  try {
    await authApiClient.post<void>("/auth/logout");
  } finally {
    clearCsrfToken();
  }
}

export async function getCurrentUser(): Promise<SessionPayload> {
  const response = await authApiClient.get<SessionPayload>("/auth/me");
  setCsrfToken(response.csrfToken);
  return response;
}

export async function updateProfile(
  input: UpdateProfileInput
): Promise<SessionPayload> {
  const response = await authApiClient.patch<SessionPayload>("/profile", input);
  if (response.csrfToken) {
    setCsrfToken(response.csrfToken);
  }
  return response;
}
