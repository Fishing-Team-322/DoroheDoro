import type { ReactNode } from "react";

export type DataTableColumn<TData> = {
  accessorKey: keyof TData & string;
  header: string;
  enableSorting?: boolean;
  cell?: (row: TData) => ReactNode;
};
