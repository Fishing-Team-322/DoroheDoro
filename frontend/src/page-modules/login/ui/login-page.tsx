"use client";

import { useEffect, useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import type { FormEvent } from "react";
import type { Locale } from "@/src/shared/config";
import {
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
    } catch {
      setLocalError("Не удалось войти. Проверьте логин и пароль.");
    } finally {
      setIsSubmitting(false);
    }
  };

  const showLoading = status === "loading";

  return (
    <main className="min-h-screen bg-[var(--background)] text-[var(--foreground)]">
      <div className="flex min-h-screen items-center justify-center px-6 py-10">
        <div className="w-full max-w-[440px] rounded-[32px] bg-[var(--surface)] px-8 py-8 shadow-[0_20px_80px_rgba(0,0,0,0.45)] sm:px-10 sm:py-10">
          <div className="mb-6">
            <h1 className="mt-3 text-3xl font-semibold tracking-tight text-[var(--card-foreground)]">
              Вход в дашборд
            </h1>
          </div>

          {showLoading ? (
            <div className="flex min-h-[180px] items-center justify-center">
              <div className="inline-flex items-center gap-3 rounded-full bg-[var(--surface-elevated)] px-4 py-2 text-sm text-[var(--muted-foreground)]">
                <Spinner size="sm" />
                Проверяем сессию...
              </div>
            </div>
          ) : step === "identifier" ? (
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

              <Button type="submit" size="lg" className="w-full">
                Продолжить
              </Button>
            </form>
          ) : (
            <form onSubmit={handlePasswordSubmit} className="space-y-5">
              <div className="space-y-2">
                <button
                  type="button"
                  onClick={handleBackToIdentifier}
                  className="inline-flex h-8 w-8 items-center justify-center rounded-full text-[var(--muted-foreground)] transition hover:bg-[var(--surface-elevated)] hover:text-[var(--foreground)]"
                  aria-label="Назад"
                >
                  <span className="text-lg leading-none">←</span>
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
              >
                Войти
              </Button>
            </form>
          )}
        </div>
      </div>
    </main>
  );
}
