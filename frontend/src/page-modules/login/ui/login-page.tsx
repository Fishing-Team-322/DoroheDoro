"use client";

import Image from "next/image";
import { useEffect, useMemo, useState, useTransition } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import type { FormEvent } from "react";
import { AnimatePresence, motion } from "motion/react";
import { type Locale } from "@/src/shared/config";
import {
  getDefaultAuthenticatedPath,
  normalizeRedirectPath,
} from "@/src/shared/lib/auth";
import { cn } from "@/src/shared/lib/cn";
import { replacePathLocale, useI18n } from "@/src/shared/lib/i18n";
import { Button, Input, Spinner } from "@/src/shared/ui";
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

const BUTTON_SIZE = 36;
const TRACK_GAP = 2;
const TRACK_PADDING = 4;
const TRACK_HEIGHT = BUTTON_SIZE + TRACK_PADDING * 2;
const TRACK_WIDTH = BUTTON_SIZE * 2 + TRACK_GAP + TRACK_PADDING * 2;
const FLAG_SIZE = 20;

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

  const [identifierReadonly, setIdentifierReadonly] = useState(true);
  const [passwordReadonly, setPasswordReadonly] = useState(true);

  const [isLocalePending, startLocaleTransition] = useTransition();

  const redirectTo = useMemo(
    () =>
      normalizeRedirectPath(searchParams.get("next"), locale) ??
      `/${locale}/overview`,
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
    setPasswordReadonly(true);
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
    setPasswordReadonly(true);
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

    startLocaleTransition(() => {
      router.replace(query ? `${nextPath}?${query}` : nextPath);
    });
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
                    <form
                      onSubmit={handleIdentifierSubmit}
                      className="space-y-5"
                      autoComplete="off"
                    >
                      <input
                        type="text"
                        name="username"
                        autoComplete="username"
                        tabIndex={-1}
                        className="hidden"
                        aria-hidden="true"
                      />
                      <input
                        type="password"
                        name="current-password"
                        autoComplete="current-password"
                        tabIndex={-1}
                        className="hidden"
                        aria-hidden="true"
                      />

                      <Input
                        id="auth_login_value"
                        name="auth_login_value"
                        type="text"
                        inputSize="lg"
                        label={dictionary.auth.login.identifierLabel}
                        autoComplete="new-password"
                        readOnly={identifierReadonly}
                        onFocus={() => setIdentifierReadonly(false)}
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
                    <form
                      onSubmit={handlePasswordSubmit}
                      className="space-y-5"
                      autoComplete="off"
                    >
                      <input
                        type="text"
                        name="username"
                        autoComplete="username"
                        tabIndex={-1}
                        className="hidden"
                        aria-hidden="true"
                      />
                      <input
                        type="password"
                        name="current-password"
                        autoComplete="current-password"
                        tabIndex={-1}
                        className="hidden"
                        aria-hidden="true"
                      />

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
                        id="auth_secret_value"
                        name="auth_secret_value"
                        type="password"
                        inputSize="lg"
                        label={dictionary.auth.login.passwordLabel}
                        autoComplete="new-password"
                        readOnly={passwordReadonly}
                        onFocus={() => setPasswordReadonly(false)}
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
              <LoginLanguageSwitch
                locale={locale}
                disabled={isLocalePending}
                onChange={handleLocaleChange}
                ariaLabel={dictionary.auth.login.localeLabel}
                russianAlt="Русский"
                englishAlt="English"
              />
            </div>
          </div>
        </motion.div>
      </div>
    </main>
  );
}

type LoginLanguageSwitchProps = {
  locale: Locale;
  disabled?: boolean;
  onChange: (locale: Locale) => void;
  ariaLabel: string;
  russianAlt: string;
  englishAlt: string;
};

function LoginLanguageSwitch({
  locale,
  disabled,
  onChange,
  ariaLabel,
  russianAlt,
  englishAlt,
}: LoginLanguageSwitchProps) {
  const normalizedLocale = String(locale).toLowerCase();
  const value = normalizedLocale === "en" ? "en" : "ru";
  const activeIndex = value === "ru" ? 0 : 1;

  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      className={cn(
        "relative inline-flex items-center rounded-[16px] border border-white/8 bg-[rgba(255,255,255,0.04)] shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]",
        disabled && "pointer-events-none opacity-60"
      )}
      style={{
        width: `${TRACK_WIDTH}px`,
        height: `${TRACK_HEIGHT}px`,
        padding: `${TRACK_PADDING}px`,
        gap: `${TRACK_GAP}px`,
      }}
    >
      <div
        aria-hidden="true"
        className="absolute top-1/2 rounded-[12px] border border-white/10 bg-[rgba(255,255,255,0.14)] shadow-[0_6px_18px_rgba(0,0,0,0.24)] transition-transform duration-250 ease-[cubic-bezier(0.22,1,0.36,1)]"
        style={{
          width: `${BUTTON_SIZE}px`,
          height: `${BUTTON_SIZE}px`,
          left: `${TRACK_PADDING}px`,
          transform: `translate(${activeIndex * (BUTTON_SIZE + TRACK_GAP)}px, -50%)`,
        }}
      />

      <LanguageFlagButton
        active={value === "ru"}
        disabled={disabled}
        imageSrc="/img/ru.png"
        imageAlt={russianAlt}
        onClick={() => onChange("ru" as Locale)}
      />

      <LanguageFlagButton
        active={value === "en"}
        disabled={disabled}
        imageSrc="/img/en.png"
        imageAlt={englishAlt}
        onClick={() => onChange("en" as Locale)}
      />
    </div>
  );
}

type LanguageFlagButtonProps = {
  active: boolean;
  disabled?: boolean;
  imageSrc: string;
  imageAlt: string;
  onClick: () => void;
};

function LanguageFlagButton({
  active,
  disabled,
  imageSrc,
  imageAlt,
  onClick,
}: LanguageFlagButtonProps) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={active}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "relative z-[1] inline-flex shrink-0 items-center justify-center rounded-[12px] transition-all duration-200",
        active ? "opacity-100" : "opacity-55 hover:opacity-85"
      )}
      style={{
        width: `${BUTTON_SIZE}px`,
        height: `${BUTTON_SIZE}px`,
      }}
    >
      <Image
        src={imageSrc}
        alt={imageAlt}
        width={FLAG_SIZE}
        height={FLAG_SIZE}
        className="rounded-full object-cover"
        style={{
          width: `${FLAG_SIZE}px`,
          height: `${FLAG_SIZE}px`,
        }}
      />
    </button>
  );
}
