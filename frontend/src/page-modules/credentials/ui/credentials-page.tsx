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

export function CredentialsPage({
  embedded = false,
}: {
  embedded?: boolean;
} = {}) {
  const { dictionary } = useI18n();
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
            loadError instanceof Error ? loadError.message : "Failed to load credentials"
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
  }, []);

  return (
    <div className={embedded ? "space-y-4" : "space-y-6"}>
      {!embedded ? (
        <PageHeader
          title="Credentials"
          description="Live credentials metadata. Secret material itself stays in Vault."
          breadcrumbs={[
            { label: dictionary.common.dashboard, href: "#" },
            { label: "Credentials" },
          ]}
        />
      ) : null}

      {loading ? <LoadingCard label="Loading credentials..." /> : null}
      {!loading && error ? <ErrorCard message={error} /> : null}

      {!loading && !error ? (
        <Card>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Kind</TableHead>
                <TableHead>Vault ref</TableHead>
                <TableHead>Updated</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {items.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4}>
                    <EmptyState
                      variant="flush"
                      title="No credential profiles"
                      description="Create credential metadata that points to Vault-backed SSH material."
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
