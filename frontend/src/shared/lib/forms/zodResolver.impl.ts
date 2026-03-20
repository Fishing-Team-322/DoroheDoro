import type { FieldErrors, Resolver } from "react-hook-form";
import type { infer as Infer, ZodTypeAny } from "zod";

const toPathKey = (path: PropertyKey[]) => path.map(String).join(".");

export function createZodResolver<TSchema extends ZodTypeAny>(
  schema: TSchema
): Resolver<Infer<TSchema>> {
  return async (values) => {
    const result = await schema.safeParseAsync(values);

    if (result.success) {
      return {
        values: result.data,
        errors: {},
      };
    }

    const fieldErrors = {} as FieldErrors<Infer<TSchema>>;
    const errorsByPath = fieldErrors as Record<
      string,
      { type: string; message: string }
    >;

    for (const issue of result.error.issues) {
      const path = toPathKey(issue.path);
      if (!path) continue;

      errorsByPath[path] = {
        type: issue.code,
        message: issue.message,
      };
    }

    return {
      values: {},
      errors: fieldErrors,
    };
  };
}
