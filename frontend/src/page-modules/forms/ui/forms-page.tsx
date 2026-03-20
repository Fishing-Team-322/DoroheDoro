"use client";

import { useState } from "react";
import { z } from "zod";
import {
  Button,
  Card,
  Checkbox,
  Input,
  Label,
  Select,
  Textarea,
} from "@/src/shared/ui";

const formSchema = z.object({
  fullName: z.string().min(2, "Имя должно содержать не менее 2 символов."),
  role: z.enum(["developer", "designer", "manager"], {
    message: "Пожалуйста, выберите роль.",
  }),
  bio: z.string().min(10, "Описание должно содержать не менее 10 символов."),
  country: z.string().min(1, "Пожалуйста, выберите страну."),
  terms: z.boolean().refine((value) => value, "Нужно принять условия."),
});

type FormValues = z.infer<typeof formSchema>;
type FormErrors = Partial<Record<keyof FormValues, string>>;

const initialValues: FormValues = {
  fullName: "",
  role: "developer",
  bio: "",
  country: "",
  terms: false,
};

const roleOptions = [
  { value: "developer", label: "Разработчик" },
  { value: "designer", label: "Дизайнер" },
  { value: "manager", label: "Менеджер" },
];

const countryOptions = [
  { value: "fr", label: "Франция" },
  { value: "de", label: "Германия" },
  { value: "es", label: "Испания" },
];

export default function FormsPage() {
  const [values, setValues] = useState<FormValues>(initialValues);
  const [errors, setErrors] = useState<FormErrors>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [result, setResult] = useState<string>("");

  const setField = <TKey extends keyof FormValues>(
    key: TKey,
    value: FormValues[TKey]
  ) => {
    setValues((current) => ({ ...current, [key]: value }));
    setErrors((current) => ({ ...current, [key]: undefined }));
  };

  const onSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setResult("");

    const parsed = formSchema.safeParse(values);

    if (!parsed.success) {
      const nextErrors: FormErrors = {};
      for (const issue of parsed.error.issues) {
        const field = issue.path[0] as keyof FormValues | undefined;
        if (field && !nextErrors[field]) {
          nextErrors[field] = issue.message;
        }
      }
      setErrors(nextErrors);
      return;
    }

    setErrors({});
    setIsSubmitting(true);
    await new Promise((resolve) => setTimeout(resolve, 900));
    setResult(JSON.stringify(parsed.data, null, 2));
    setIsSubmitting(false);
  };

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-2xl flex-col gap-6 p-8">
      <h1 className="text-3xl font-semibold">Демо форм</h1>

      <Card>
        <form onSubmit={onSubmit} className="space-y-5">
          <div className="space-y-2">
            <Label htmlFor="fullName">Полное имя</Label>
            <Input
              id="fullName"
              placeholder="Иван Иванов"
              value={values.fullName}
              onChange={(event) => setField("fullName", event.target.value)}
              disabled={isSubmitting}
            />
            {errors.fullName ? (
              <p className="text-sm text-red-600">{errors.fullName}</p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="role">Роль</Label>
            <Select
              id="role"
              value={values.role}
              onChange={(event) =>
                setField("role", event.target.value as FormValues["role"])
              }
              disabled={isSubmitting}
              options={roleOptions}
              placeholder="Выберите роль"
              selectSize="md"
            />
            {errors.role ? (
              <p className="text-sm text-red-600">{errors.role}</p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="country">Страна</Label>
            <Select
              id="country"
              value={values.country}
              onChange={(event) => setField("country", event.target.value)}
              disabled={isSubmitting}
              options={countryOptions}
              placeholder="Выберите страну"
              selectSize="md"
            />
            {errors.country ? (
              <p className="text-sm text-red-600">{errors.country}</p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Label htmlFor="bio">О себе</Label>
            <Textarea
              id="bio"
              rows={4}
              placeholder="Расскажите немного о себе"
              value={values.bio}
              onChange={(event) => setField("bio", event.target.value)}
              disabled={isSubmitting}
            />
            {errors.bio ? (
              <p className="text-sm text-red-600">{errors.bio}</p>
            ) : null}
          </div>

          <div className="space-y-2">
            <Checkbox
              label="Я принимаю условия использования"
              checked={values.terms}
              onChange={(event) => setField("terms", event.target.checked)}
              disabled={isSubmitting}
            />
            {errors.terms ? (
              <p className="text-sm text-red-600">{errors.terms}</p>
            ) : null}
          </div>

          <Button type="submit" loading={isSubmitting} disabled={isSubmitting}>
            Отправить
          </Button>
        </form>
      </Card>

      {result ? (
        <pre className="overflow-auto rounded-lg border border-zinc-800 bg-zinc-950 p-4 text-sm text-zinc-100">
          {result}
        </pre>
      ) : null}
    </main>
  );
}
