"use client";

import { useEffect, useMemo, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import type { FormEvent } from "react";
import { AnimatePresence, motion } from "motion/react";
import { locales, type Locale } from "@/src/shared/config";
import {
  getDefaultAuthenticatedPath,
  normalizeRedirectPath,
} from "@/src/shared/lib/auth";
import { replacePathLocale, useI18n } from "@/src/shared/lib/i18n";
import { Button, Input, Select, Spinner } from "@/src/shared/ui";
import { useAuth } from "@/src/features/auth";

type LoginPageProps = {
  locale: Locale;
};

type Step = "identifier" | "password";

const cardTransition = {
  duration: 0.36,
  ease: [0.22, 1, 0.36, 1] as const,
};

const stepTransition = {
  duration: 0.18,
  ease: [0.22, 1, 0.36, 1] as const,
};

const stepVariants = {
  initial: {
    opacity: 0,
    y: 10,
    scale: 0.985,
  },
  animate: {
    opacity: 1,
    y: 0,
    scale: 1,
  },
  exit: {
    opacity: 0,
    y: -6,
    scale: 0.985,
  },
};

export function LoginPage({ locale }: LoginPageProps) {
  const pathname = usePathname();
  const router = useRouter();
  const searchParams = useSearchParams();
  const { dictionary } = useI18n();
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
      setLocalError(dictionary.auth.login.errors.identifierRequired);
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
      setLocalError(dictionary.auth.login.errors.passwordRequired);
      return;
    }

    if (!login) {
      setLocalError(dictionary.auth.login.errors.loginUnavailable);
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
      setLocalError(dictionary.auth.login.errors.invalidCredentials);
    } finally {
      setIsSubmitting(false);
    }
  };

  const showLoading = status === "loading";
  const isIdentifierEmpty = !identifier.trim();
  const isPasswordEmpty = !password.trim();

  const localeOptions = locales.map((option) => ({
    value: option,
    label: dictionary.auth.login.localeOptions[option],
  }));

  const handleLocaleChange = (nextLocale: Locale) => {
    if (nextLocale === locale) {
      return;
    }

    const params = new URLSearchParams(searchParams.toString());
    const nextRedirect = normalizeRedirectPath(params.get("next"), locale);

    if (nextRedirect) {
      params.set("next", replacePathLocale(nextRedirect, nextLocale));
    } else {
      params.delete("next");
    }

    const nextPath = replacePathLocale(pathname, nextLocale);
    const query = params.toString();

    router.replace(query ? `${nextPath}?${query}` : nextPath);
  };

  return (
    <main className="min-h-screen bg-[var(--background)] text-[var(--foreground)]">
      <div className="flex min-h-screen items-center justify-center px-6 py-10">
        <motion.div
          initial={{ opacity: 0, y: 24, scale: 0.985 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={cardTransition}
          className="flex min-h-[420px] w-full max-w-[440px] flex-col rounded-[32px] bg-[var(--surface)] px-8 py-8 shadow-[0_20px_80px_rgba(0,0,0,0.45)] sm:px-10 sm:py-10"
        >
          <div className="mb-6">
            <h1 className="pb-2 text-4xl font-semibold tracking-tight text-[#f3f3f3]">
              {dictionary.auth.login.title}
            </h1>
          </div>

          {showLoading ? (
            <div className="flex min-h-[260px] items-center justify-center">
              <motion.div
                initial={{ opacity: 0, scale: 0.97 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ duration: 0.18 }}
                className="inline-flex items-center gap-3 rounded-full bg-[var(--surface-elevated)] px-4 py-2 text-sm text-[var(--muted-foreground)]"
              >
                <Spinner size="sm" />
                {dictionary.auth.login.checkingSession}
              </motion.div>
            </div>
          ) : (
            <div className="relative min-h-[260px]">
              <AnimatePresence mode="wait" initial={false}>
                {step === "identifier" ? (
                  <motion.div
                    key="identifier-step"
                    variants={stepVariants}
                    initial="initial"
                    animate="animate"
                    exit="exit"
                    transition={stepTransition}
                    className="absolute inset-0"
                  >
                    <form onSubmit={handleIdentifierSubmit} className="space-y-5">
                      <Input
                        id="identifier"
                        name="identifier"
                        type="text"
                        inputSize="lg"
                        label={dictionary.auth.login.identifierLabel}
                        autoComplete="off"
                        value={identifier}
                        onChange={(e) => {
                          setIdentifier(e.target.value);
                          if (localError) setLocalError("");
                        }}
                        error={localError || undefined}
                        className="!border-transparent !shadow-none hover:!border-transparent focus:!border-transparent focus:!shadow-none"
                      />

                      <Button
                        type="submit"
                        size="lg"
                        className="w-full"
                        disabled={isIdentifierEmpty}
                      >
                        {dictionary.auth.login.continue}
                      </Button>
                    </form>
                  </motion.div>
                ) : (
                  <motion.div
                    key="password-step"
                    variants={stepVariants}
                    initial="initial"
                    animate="animate"
                    exit="exit"
                    transition={stepTransition}
                    className="absolute inset-0"
                  >
                    <form onSubmit={handlePasswordSubmit} className="space-y-5" autoComplete="off">
                      <div className="space-y-3">
                        <button
                          type="button"
                          onClick={handleBackToIdentifier}
                          aria-label={dictionary.auth.login.back}
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
                        label={dictionary.auth.login.passwordLabel}
                        autoComplete="current-password"
                        value={password}
                        onChange={(e) => {
                          setPassword(e.target.value);
                          if (localError) setLocalError("");
                        }}
                        error={localError || undefined}
                        className="!border-transparent !shadow-none hover:!border-transparent focus:!border-transparent focus:!shadow-none"
                      />

                      <Button
                        type="submit"
                        size="lg"
                        loading={isSubmitting}
                        className="w-full"
                        disabled={isPasswordEmpty}
                      >
                        {dictionary.auth.login.submit}
                      </Button>
                    </form>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          )}

          <div className="mt-auto pt-8 text-center">
            <p className="text-xs leading-5 text-[var(--muted-foreground)]/50">
              {dictionary.auth.login.footerTeam}
            </p>
            <p className="text-xs leading-5 text-[var(--muted-foreground)]/50">
              {dictionary.auth.login.footerProduct}
            </p>

            <div className="mt-4 flex justify-center">
              <div className="w-[132px] shrink-0">
                <Select
                  id="login-locale"
                  value={locale}
                  onChange={(event) =>
                    handleLocaleChange(event.target.value as Locale)
                  }
                  options={localeOptions}
                  selectSize="sm"
                  aria-label={dictionary.auth.login.localeLabel}
                />
              </div>
            </div>
          </div>
        </motion.div>
      </div>
    </main>
  );
}