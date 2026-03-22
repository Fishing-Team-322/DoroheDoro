"use client";

import { useEffect, useState } from "react";
import { useI18n } from "@/src/shared/lib/i18n";
import { formatRelativeLabel } from "@/src/shared/lib/dashboard";
import {
  listHostGroups,
  listHosts,
  type HostGroupItem,
  type HostItem,
} from "@/src/shared/lib/runtime-api";
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
import { ErrorCard, JsonValue, LoadingCard } from "@/src/page-modules/common/ui/runtime-state";

const copyByLocale = {
  en: {
    loadError: "Failed to load inventory",
    title: "Inventory",
    description: "Live host inventory and host groups from control-plane.",
    loading: "Loading inventory...",
    hosts: {
      title: "Hosts",
      columns: {
        hostname: "Hostname",
        ip: "IP",
        remoteUser: "Remote user",
        updated: "Updated",
      },
      emptyTitle: "No hosts",
      emptyDescription:
        "Create hosts through WEB or the API to populate inventory.",
    },
    groups: {
      title: "Host groups",
      columns: {
        name: "Name",
        description: "Description",
        members: "Members",
        updated: "Updated",
      },
      emptyTitle: "No host groups",
      emptyDescription:
        "Create host groups to target deployments by inventory slice.",
    },
    inspector: {
      title: "Host inspector",
      createdPrefix: "Created",
      emptyTitle: "No host selected",
      emptyDescription:
        "Pick a host from the table to inspect its labels and access metadata.",
    },
  },
  ru: {
    loadError: "Не удалось загрузить inventory",
    title: "Инвентарь",
    description: "Живой инвентарь хостов и групп хостов из control-plane.",
    loading: "Загрузка инвентаря...",
    hosts: {
      title: "Хосты",
      columns: {
        hostname: "Hostname",
        ip: "IP",
        remoteUser: "Пользователь",
        updated: "Обновлено",
      },
      emptyTitle: "Хостов нет",
      emptyDescription:
        "Создайте хосты через WEB или API, чтобы заполнить инвентарь.",
    },
    groups: {
      title: "Группы хостов",
      columns: {
        name: "Имя",
        description: "Описание",
        members: "Участники",
        updated: "Обновлено",
      },
      emptyTitle: "Групп хостов нет",
      emptyDescription:
        "Создайте группы хостов, чтобы таргетировать раскатки по срезам инвентаря.",
    },
    inspector: {
      title: "Инспектор хоста",
      createdPrefix: "Создан",
      emptyTitle: "Хост не выбран",
      emptyDescription:
        "Выберите хост в таблице, чтобы посмотреть его labels и access metadata.",
    },
  },
} as const;

export function InventoryPage({ embedded = false }: { embedded?: boolean } = {}) {
  const { dictionary, locale } = useI18n();
  const copy = copyByLocale[locale];
  const [hosts, setHosts] = useState<HostItem[]>([]);
  const [hostGroups, setHostGroups] = useState<HostGroupItem[]>([]);
  const [selectedHostId, setSelectedHostId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    async function load() {
      setLoading(true);
      setError(null);
      try {
        const [hostsResponse, groupsResponse] = await Promise.all([
          listHosts(),
          listHostGroups(),
        ]);
        if (cancelled) {
          return;
        }
        setHosts(hostsResponse.items);
        setHostGroups(groupsResponse.items);
        setSelectedHostId((current) => current ?? hostsResponse.items[0]?.host_id ?? null);
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

  const selectedHost = hosts.find((item) => item.host_id === selectedHostId) ?? null;

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
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)]">
          <div className="space-y-4">
            <Card>
              <div className="space-y-3">
                <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                  {copy.hosts.title}
                </h2>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{copy.hosts.columns.hostname}</TableHead>
                      <TableHead>{copy.hosts.columns.ip}</TableHead>
                      <TableHead>{copy.hosts.columns.remoteUser}</TableHead>
                      <TableHead>{copy.hosts.columns.updated}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {hosts.length === 0 ? (
                      <TableRow>
                        <TableCell colSpan={4}>
                          <EmptyState
                            variant="flush"
                            title={copy.hosts.emptyTitle}
                            description={copy.hosts.emptyDescription}
                          />
                        </TableCell>
                      </TableRow>
                    ) : (
                      hosts.map((host) => (
                        <TableRow
                          key={host.host_id}
                          className={
                            host.host_id === selectedHostId
                              ? "bg-[color:rgba(56,189,248,0.08)]"
                              : undefined
                          }
                          onClick={() => setSelectedHostId(host.host_id)}
                        >
                          <TableCell className="font-medium text-[color:var(--foreground)]">
                            {host.hostname}
                          </TableCell>
                          <TableCell>{host.ip}:{host.ssh_port}</TableCell>
                          <TableCell>{host.remote_user}</TableCell>
                          <TableCell>{formatRelativeLabel(host.updated_at, locale)}</TableCell>
                        </TableRow>
                      ))
                    )}
                  </TableBody>
                </Table>
              </div>
            </Card>

            <Card>
              <div className="space-y-3">
                <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                  {copy.groups.title}
                </h2>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{copy.groups.columns.name}</TableHead>
                      <TableHead>{copy.groups.columns.description}</TableHead>
                      <TableHead>{copy.groups.columns.members}</TableHead>
                      <TableHead>{copy.groups.columns.updated}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {hostGroups.length === 0 ? (
                      <TableRow>
                        <TableCell colSpan={4}>
                          <EmptyState
                            variant="flush"
                            title={copy.groups.emptyTitle}
                            description={copy.groups.emptyDescription}
                          />
                        </TableCell>
                      </TableRow>
                    ) : (
                      hostGroups.map((group) => (
                        <TableRow key={group.host_group_id}>
                          <TableCell className="font-medium text-[color:var(--foreground)]">
                            {group.name}
                          </TableCell>
                          <TableCell>{group.description || "—"}</TableCell>
                          <TableCell>{group.members.length}</TableCell>
                          <TableCell>{formatRelativeLabel(group.updated_at, locale)}</TableCell>
                        </TableRow>
                      ))
                    )}
                  </TableBody>
                </Table>
              </div>
            </Card>
          </div>

          <Card>
            <div className="space-y-3">
                <h2 className="text-base font-semibold text-[color:var(--foreground)]">
                  {copy.inspector.title}
                </h2>
              {selectedHost ? (
                <>
                  <div className="rounded-lg border border-[color:var(--border)] bg-[color:var(--surface)] p-3">
                    <p className="text-lg font-semibold text-[color:var(--foreground)]">
                      {selectedHost.hostname}
                    </p>
                    <p className="mt-1 text-sm text-[color:var(--muted-foreground)]">
                      {selectedHost.ip}:{selectedHost.ssh_port} via {selectedHost.remote_user}
                    </p>
                    <p className="mt-2 text-xs uppercase tracking-[0.12em] text-[color:var(--muted-foreground)]">
                      {copy.inspector.createdPrefix}{" "}
                      {formatRelativeLabel(selectedHost.created_at, locale)}
                    </p>
                  </div>
                  <JsonValue value={selectedHost.labels} />
                </>
              ) : (
                <EmptyState
                  variant="flush"
                  title={copy.inspector.emptyTitle}
                  description={copy.inspector.emptyDescription}
                />
              )}
            </div>
          </Card>
        </section>
      ) : null}
    </div>
  );
}
