"use client";

import { useEffect, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
import { listCredentials, type CredentialItem } from "@/src/shared/lib/runtime-api";
import {
  Card,
  EmptyState,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/src/shared/ui";
import { PageHeader } from "@/src/widgets/dashboard-layout";
import { ErrorCard, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";

const copyByLocale = {
  en: {
    loadError: "Failed to load credentials",
    title: "Credentials",
    description:
      "Live credentials metadata. Secret material itself stays in Vault.",
    loading: "Loading credentials...",
    columns: {
      name: "Name",
      kind: "Kind",
      vaultRef: "Vault ref",
      updated: "Updated",
    },
    emptyTitle: "No credential profiles",
    emptyDescription:
      "Create credential metadata that points to Vault-backed SSH material.",
  },
  ru: {
    loadError: "Не удалось загрузить credential profiles",
    title: "Доступы",
    description:
      "Живые метаданные credential-профилей. Секретный материал остается в Vault.",
    loading: "Загрузка credential-профилей...",
    columns: {
      name: "Имя",
      kind: "Тип",
      vaultRef: "Ссылка в Vault",
      updated: "Обновлено",
    },
    emptyTitle: "Нет профилей доступов",
    emptyDescription:
      "Создайте метаданные доступов, которые указывают на SSH-материал в Vault.",
  },
} as const;

export function CredentialsPage({
  embedded = false,
}: {
  embedded?: boolean;
} = {}) {
  const { dictionary, locale } = useI18n();
  const copy = copyByLocale[locale];
  const [items, setItems] = useState<CredentialItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const response = await listCredentials();
        if (!cancelled) {
          setItems(response.items);
        }
      } catch (loadError) {
        if (!cancelled) {
          setError(
            loadError instanceof Error ? loadError.message : copy.loadError
          );
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void load();
    return () => {
      cancelled = true;
    };
  }, [copy.loadError]);

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title={copy.title}
          description={copy.description}
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: copy.title },
          ]}
        />
      ) : null}

      {loading ? <LoadingCard label={copy.loading} /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <Card>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{copy.columns.name}</TableHead>
                <TableHead>{copy.columns.kind}</TableHead>
                <TableHead>{copy.columns.vaultRef}</TableHead>
                <TableHead>{copy.columns.updated}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4}>
                    <EmptyState
                      variant="flush"
                      title={copy.emptyTitle}
                      description={copy.emptyDescription}
                    />
                  </TableCell>
                </TableRow>
              ) : (
                items.map((item) => (
                  <TableRow key={item.credentials_profile_id}>
                    <TableCell className="font-medium text-[color:var(--foreground)]">
                      {item.name}
                    </TableCell>
                    <TableCell>{item.kind}</TableCell>
                    <TableCell>{item.vault_ref}</TableCell>
                    <TableCell>{item.updated_at}</TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </Card>
      ) : null}
    </div>
  );
}
