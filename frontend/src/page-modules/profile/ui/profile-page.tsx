"use client";

import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { useAuth } from "@/src/features/auth";
import { createZodResolver } from "@/src/shared/lib/forms";
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

const profileSchema = z.object({
  displayName: z
    .string()
    .trim()
    .min(2, "Display name must be at least 2 characters")
    .max(80, "Display name must be 80 characters or fewer"),
});

type ProfileFormValues = z.infer<typeof profileSchema>;

export function ProfilePage() {
  const { user, updateProfile } = useAuth();
  const [formError, setFormError] = useState<string>();
  const [successMessage, setSuccessMessage] = useState<string>();

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
      setSuccessMessage("Profile updated successfully.");
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Unable to update profile.";
      setFormError(message);
    }
  });

  return (
    <main className="space-y-6">
      <PageHeader
        title="Profile"
        description="Update your visible profile data. This request uses the CSRF-aware fetch wrapper."
      />

      <Section className="border-t-0 py-0">
        <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
          <Card className="space-y-5">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold text-[color:var(--foreground)]">
                Edit profile
              </h2>
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Saving sends a `PATCH /profile` request with the current CSRF token.
              </p>
            </div>

            <form className="space-y-5" onSubmit={onSubmit}>
              <FormField>
                <FormLabel htmlFor="displayName">Display name</FormLabel>
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
                Save changes
              </Button>
            </form>
          </Card>

          <Card className="space-y-4">
            <div className="space-y-1">
              <h2 className="text-lg font-semibold text-[color:var(--foreground)]">
                Current account
              </h2>
              <p className="text-sm text-[color:var(--muted-foreground)]">
                Current user data loaded from `GET /auth/me`.
              </p>
            </div>

            <dl className="space-y-3 text-sm">
              <div className="rounded-2xl border border-[color:var(--border)] bg-[color:var(--background)] px-4 py-3">
                <dt className="text-[color:var(--muted-foreground)]">Email</dt>
                <dd className="mt-1 font-medium text-[color:var(--foreground)]">
                  {user.email}
                </dd>
              </div>

              <div className="rounded-2xl border border-[color:var(--border)] bg-[color:var(--background)] px-4 py-3">
                <dt className="text-[color:var(--muted-foreground)]">Login</dt>
                <dd className="mt-1 font-medium text-[color:var(--foreground)]">
                  {user.login}
                </dd>
              </div>

              <div className="rounded-2xl border border-[color:var(--border)] bg-[color:var(--background)] px-4 py-3">
                <dt className="text-[color:var(--muted-foreground)]">Display name</dt>
                <dd className="mt-1 font-medium text-[color:var(--foreground)]">
                  {user.displayName}
                </dd>
              </div>
            </dl>
          </Card>
        </div>
      </Section>
    </main>
  );
}
