"use client";

import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { useAuth } from "@/src/features/auth";
import { createZodResolver } from "@/src/shared/lib/forms";
import { useI18n } from "@/src/shared/lib/i18n";
import {
  Button,
  Card,
  FormControl,
  FormError,
  FormField,
  FormLabel,
  Input,
} from "@/src/shared/ui";
import { PageHeader, Section } from "@/src/widgets/dashboard-layout";

type ProfileFormValues = {
  displayName: string;
};

export function ProfilePage() {
  const { dictionary } = useI18n();
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
      const message = error instanceof Error ? error.message : copy.fallbackError;
      setFormError(message);
    }
  });

  return (
    <div className="space-y-6">
      <PageHeader title={copy.title} description={copy.description} />

      <Section className="border-t-0 py-0">
        <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
          <Card className="space-y-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold text-[color:var(--foreground)]">
                {copy.editTitle}
              </h2>
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.editDescription}
              </p>
            </div>

            <form className="space-y-5" onSubmit={onSubmit}>
              <FormField>
                <FormLabel htmlFor="displayName">{copy.displayNameLabel}</FormLabel>
                <FormControl hasError={Boolean(form.formState.errors.displayName)}>
                  <Input id="displayName" {...form.register("displayName")} />
                </FormControl>
                <FormError message={form.formState.errors.displayName?.message} />
              </FormField>

              <FormError message={formError} />

              {successMessage ? (
                <p className="text-sm text-emerald-600">{successMessage}</p>
              ) : null}

              <Button
                type="submit"
                loading={form.formState.isSubmitting}
                className="w-full sm:w-auto"
              >
                {copy.save}
              </Button>
            </form>
          </Card>

          <Card className="space-y-4">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold text-[color:var(--foreground)]">
                {copy.currentAccountTitle}
              </h2>
              <p className="text-sm text-[color:var(--muted-foreground)]">
                {copy.currentAccountDescription}
              </p>
            </div>

            <dl className="space-y-3 text-sm">
              <div className="rounded-2xl border border-[color:var(--border)] bg-[color:var(--background)] px-4 py-3">
                <dt className="text-[color:var(--muted-foreground)]">{copy.fields.email}</dt>
                <dd className="mt-1 font-medium text-[color:var(--foreground)]">
                  {user.email}
                </dd>
              </div>

              <div className="rounded-2xl border border-[color:var(--border)] bg-[color:var(--background)] px-4 py-3">
                <dt className="text-[color:var(--muted-foreground)]">{copy.fields.login}</dt>
                <dd className="mt-1 font-medium text-[color:var(--foreground)]">
                  {user.login}
                </dd>
              </div>

              <div className="rounded-2xl border border-[color:var(--border)] bg-[color:var(--background)] px-4 py-3">
                <dt className="text-[color:var(--muted-foreground)]">
                  {copy.fields.displayName}
                </dt>
                <dd className="mt-1 font-medium text-[color:var(--foreground)]">
                  {user.displayName}
                </dd>
              </div>
            </dl>
          </Card>
        </div>
      </Section>
    </div>
  );
}
