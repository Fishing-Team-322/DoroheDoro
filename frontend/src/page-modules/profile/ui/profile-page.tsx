"use client";

import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { LogoutButton, useAuth } from "@/src/features/auth";
import { createZodResolver } from "@/src/shared/lib/forms";
import { useI18n } from "@/src/shared/lib/i18n";
import { Button, FormError, FormField, Input } from "@/src/shared/ui";
import { Section } from "@/src/widgets/dashboard-layout";

type ProfileFormValues = {
  displayName: string;
};

export function ProfilePage() {
  const { dictionary, locale } = useI18n();
  const copy = dictionary.profile;
  const { user, updateProfile } = useAuth();

  const [formError, setFormError] = useState<string>();
  const [successMessage, setSuccessMessage] = useState<string>();

  const profileSchema = z.object({
    displayName: z
      .string()
      .trim()
      .min(2, copy.validation.displayNameMin)
      .max(80, copy.validation.displayNameMax),
  });

  const form = useForm<ProfileFormValues>({
    defaultValues: {
      displayName: user?.displayName ?? "",
    },
    resolver: createZodResolver(profileSchema),
  });

  useEffect(() => {
    if (!user) {
      return;
    }

    form.reset({
      displayName: user.displayName,
    });
  }, [form, user]);

  if (!user) {
    return null;
  }

  const onSubmit = form.handleSubmit(async (values) => {
    setFormError(undefined);
    setSuccessMessage(undefined);

    try {
      const updatedUser = await updateProfile(values);
      form.reset({
        displayName: updatedUser.displayName,
      });
      setSuccessMessage(copy.success);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : copy.fallbackError;
      setFormError(message);
    }
  });

  return (
    <div className="space-y-6">
      <Section className="border-t-0 py-0">
        <div className="w-full">
          <div className="rounded-[28px] border border-[color:var(--border)] bg-[color:var(--surface)] px-6 py-6 sm:px-8 sm:py-8 lg:px-10 lg:py-10">
            <div className="border-b border-[color:var(--border)] pb-6">
              <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                <span className="text-4xl font-semibold tracking-tight text-[color:var(--foreground)]">
                  {user.login}
                </span>
                <span className="text-4xl text-[#3d3d3d]">{user.email}</span>
              </div>
            </div>

            <div className="py-8">
              <dl>
                <div className="grid grid-cols-1 gap-2 py-5 sm:grid-cols-[260px_minmax(0,1fr)] sm:items-center sm:gap-6">
                  <dt className="text-xl text-[color:var(--muted-foreground)]">
                    {copy.fields.displayName}
                  </dt>
                  <dd className="min-w-0 text-xl font-medium text-[color:var(--foreground)]">
                    {user.displayName}
                  </dd>
                </div>

                <div className="grid grid-cols-1 gap-2 py-5 sm:grid-cols-[260px_minmax(0,1fr)] sm:items-center sm:gap-6">
                  <dt className="text-xl text-[color:var(--muted-foreground)]">
                    {copy.fields.login}
                  </dt>
                  <dd className="min-w-0 text-xl font-medium text-[color:var(--foreground)]">
                    {user.login}
                  </dd>
                </div>

                <div className="grid grid-cols-1 gap-2 py-5 sm:grid-cols-[260px_minmax(0,1fr)] sm:items-center sm:gap-6">
                  <dt className="text-xl text-[color:var(--muted-foreground)]">
                    {copy.fields.email}
                  </dt>
                  <dd className="min-w-0 break-all text-xl font-medium text-[color:var(--foreground)]">
                    {user.email}
                  </dd>
                </div>
              </dl>
            </div>

            <div className="border-t border-[color:var(--border)] pt-8">
              <form onSubmit={onSubmit}>
                <div className="grid grid-cols-1 gap-4 py-2 sm:grid-cols-[260px_minmax(0,1fr)] sm:items-start sm:gap-6">
                  <div className="space-y-1">
                    <h2 className="text-xl font-medium text-[color:var(--foreground)]">
                      {copy.editTitle}
                    </h2>
                    <p className="text-base leading-6 text-[color:var(--muted-foreground)]">
                      {copy.editDescription}
                    </p>
                  </div>

                  <div className="min-w-0 max-w-xl space-y-4">
                    <FormField>
                      <Input
                        id="displayName"
                        label={copy.displayNameLabel}
                        error={form.formState.errors.displayName?.message}
                        className="!text-lg"
                        {...form.register("displayName")}
                      />
                    </FormField>

                    <FormError message={formError} />

                    {successMessage ? (
                      <p className="text-sm font-medium text-emerald-600">
                        {successMessage}
                      </p>
                    ) : null}

                    <div className="flex flex-col gap-3 pt-1 sm:flex-row sm:items-center">
                      <Button
                        type="submit"
                        loading={form.formState.isSubmitting}
                        className="h-11 px-5"
                      >
                        {copy.save}
                      </Button>

                      <LogoutButton
                        locale={locale}
                        variant="ghost"
                        size="sm"
                        className="h-11 px-5"
                      >
                        {dictionary.auth.logout.full}
                      </LogoutButton>
                    </div>
                  </div>
                </div>
              </form>
            </div>
          </div>
        </div>
      </Section>
    </div>
  );
}
