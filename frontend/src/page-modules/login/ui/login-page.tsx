"use client";

import { useEffect, useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import type { FormEvent } from "react";
import type { Locale } from "@/src/shared/config";
import {
  getLoginErrorMessage,
  getDefaultAuthenticatedPath,
  normalizeRedirectPath,
} from "@/src/shared/lib/auth";
import { Button, Input, Spinner } from "@/src/shared/ui";
import { useAuth } from "@/src/features/auth";

type LoginPageProps = {
  locale: Locale;
};

type Step = "identifier" | "password";

export function LoginPage({ locale }: LoginPageProps) {
  const router = useRouter();
  const searchParams = useSearchParams();

  const { status, login } = useAuth();

  const [step, setStep] = useState<Step>("identifier");
  const [identifier, setIdentifier] = useState("");
  const [password, setPassword] = useState("");
  const [localError, setLocalError] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const redirectTo = useMemo(
    () =>
      normalizeRedirectPath(searchParams.get("next"), locale) ??
      getDefaultAuthenticatedPath(locale),
    [searchParams, locale]
  );

  useEffect(() => {
    if (status === "authenticated") {
      router.replace(redirectTo);
    }
  }, [redirectTo, router, status]);

  const handleBackToIdentifier = () => {
    setPassword("");
    setLocalError("");
    setStep("identifier");
  };

  const handleIdentifierSubmit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    const value = identifier.trim();

    if (!value) {
      setLocalError("Введите логин или email.");
      return;
    }

    setIdentifier(value);
    setPassword("");
    setLocalError("");
    setStep("password");
  };

  const handlePasswordSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    if (!password.trim()) {
      setLocalError("Введите пароль.");
      return;
    }

    if (!login) {
      setLocalError("Функция входа недоступна. Проверь useAuth.");
      return;
    }

    try {
      setLocalError("");
      setIsSubmitting(true);

      await login({
        identifier: identifier.trim(),
        password,
      });

      router.replace(redirectTo);
    } catch (error) {
      console.error("Login failed", error);
      setLocalError(getLoginErrorMessage(error));
    } finally {
      setIsSubmitting(false);
    }
  };

  const showLoading = status === "loading";
  const isIdentifierEmpty = !identifier.trim();
  const isPasswordEmpty = !password.trim();

  return (
    <main className="min-h-screen bg-[var(--background)] text-[var(--foreground)]">
      <div className="flex min-h-screen items-center justify-center px-6 py-10">
        <div className="min-h-[420px] w-full max-w-[440px] rounded-[32px] bg-[var(--surface)] px-8 py-8 shadow-[0_20px_80px_rgba(0,0,0,0.45)] sm:px-10 sm:py-10">
          <div className="mb-6">
            <h1 className="mt-3 text-3xl font-semibold tracking-tight text-[var(--card-foreground)]">
              Вход в дашборд
            </h1>
          </div>

          {showLoading ? (
            <div className="flex min-h-[260px] items-center justify-center">
              <div className="inline-flex items-center gap-3 rounded-full bg-[var(--surface-elevated)] px-4 py-2 text-sm text-[var(--muted-foreground)]">
                <Spinner size="sm" />
                Проверяем сессию...
              </div>
            </div>
          ) : step === "identifier" ? (
            <div className="min-h-[260px]">
              <form onSubmit={handleIdentifierSubmit} className="space-y-5">
                <Input
                  id="identifier"
                  name="identifier"
                  type="text"
                  inputSize="lg"
                  label="Логин или email"
                  autoComplete="username"
                  value={identifier}
                  onChange={(e) => {
                    setIdentifier(e.target.value);
                    if (localError) setLocalError("");
                  }}
                  error={localError || undefined}
                  className="!border-transparent hover:!border-transparent focus:!border-transparent !shadow-none focus:!shadow-none"
                />

                <Button
                  type="submit"
                  size="lg"
                  className="w-full"
                  disabled={isIdentifierEmpty}
                >
                  Продолжить
                </Button>
              </form>
            </div>
          ) : (
            <div className="min-h-[260px]">
              <form onSubmit={handlePasswordSubmit} className="space-y-5">
                <div className="space-y-3">
                  <button
                    type="button"
                    onClick={handleBackToIdentifier}
                    aria-label="Назад"
                    className="
                      inline-flex h-11 min-w-[64px] select-none items-center justify-center
                      rounded-xl bg-[var(--surface-elevated)] px-5
                      cursor-pointer transition-all duration-200 ease-out
                      hover:bg-[rgba(255,255,255,0.12)]
                      hover:scale-[1.02]
                      active:scale-[0.98]
                      focus:outline-none
                      focus-visible:ring-2
                      focus-visible:ring-[rgba(255,255,255,0.45)]
                      focus-visible:ring-offset-2
                      focus-visible:ring-offset-[var(--surface)]
                    "
                  >
                    <svg
                      xmlns="http://www.w3.org/2000/svg"
                      viewBox="0 0 24 24"
                      className="h-6 w-6 fill-[rgba(255,255,255,0.62)]"
                      aria-hidden="true"
                    >
                      <path d="M10.5 6.5a1 1 0 0 1 0 1.414L7.414 11H18a1 1 0 1 1 0 2H7.414l3.086 3.086a1 1 0 0 1-1.414 1.414l-4.75-4.75a1.06 1.06 0 0 1 0-1.5l4.75-4.75a1 1 0 0 1 1.414 0Z" />
                    </svg>
                  </button>

                  <p className="truncate text-base font-medium text-[var(--card-foreground)]">
                    {identifier}
                  </p>
                </div>

                <Input
                  id="password"
                  name="password"
                  type="password"
                  inputSize="lg"
                  label="Пароль"
                  autoComplete="current-password"
                  value={password}
                  onChange={(e) => {
                    setPassword(e.target.value);
                    if (localError) setLocalError("");
                  }}
                  error={localError || undefined}
                  className="!border-transparent hover:!border-transparent focus:!border-transparent !shadow-none focus:!shadow-none"
                />

                <Button
                  type="submit"
                  size="lg"
                  loading={isSubmitting}
                  className="w-full"
                  disabled={isPasswordEmpty}
                >
                  Войти
                </Button>
              </form>
            </div>
          )}
        </div>
      </div>
    </main>
  );
}
