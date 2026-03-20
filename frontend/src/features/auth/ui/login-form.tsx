"use client";

import { useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { createZodResolver } from "@/src/shared/lib/forms";
import {
  Button,
  FormControl,
  FormError,
  FormField,
  FormLabel,
  Input,
} from "@/src/shared/ui";
import { useAuth } from "../model/use-auth";

const loginSchema = z.object({
  identifier: z.string().trim().min(1, "Email or login is required"),
  password: z.string().min(1, "Password is required"),
});

type LoginFormValues = z.infer<typeof loginSchema>;

export function LoginForm() {
  const { login } = useAuth();
  const [formError, setFormError] = useState<string>();

  const form = useForm<LoginFormValues>({
    defaultValues: {
      identifier: "",
      password: "",
    },
    resolver: createZodResolver(loginSchema),
  });

  const onSubmit = form.handleSubmit(async (values) => {
    setFormError(undefined);

    try {
      await login(values);
    } catch (error) {
      const message =
        error instanceof Error ? error.message : "Unable to sign in.";
      setFormError(message);
    }
  });

  return (
    <form className="space-y-5" onSubmit={onSubmit}>
      <FormField>
        <FormLabel htmlFor="identifier">Email or login</FormLabel>
        <FormControl hasError={Boolean(form.formState.errors.identifier)}>
          <Input
            id="identifier"
            autoComplete="username"
            placeholder="you@company.com"
            {...form.register("identifier")}
          />
        </FormControl>
        <FormError message={form.formState.errors.identifier?.message} />
      </FormField>

      <FormField>
        <FormLabel htmlFor="password">Password</FormLabel>
        <FormControl hasError={Boolean(form.formState.errors.password)}>
          <Input
            id="password"
            type="password"
            autoComplete="current-password"
            placeholder="Enter your password"
            {...form.register("password")}
          />
        </FormControl>
        <FormError message={form.formState.errors.password?.message} />
      </FormField>

      <FormError message={formError} />

      <Button
        type="submit"
        className="w-full"
        loading={form.formState.isSubmitting}
      >
        Sign in
      </Button>
    </form>
  );
}
