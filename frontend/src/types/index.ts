export type SslMode = "disable" | "require" | "verify-full";

export interface ConnectionInfo {
  host: string;
  port: number;
  username: string;
  password: string;
  database: string;
  ssl_mode?: SslMode;
  ssl_root_cert?: string | null;
  ssl_cert?: string | null;
  ssl_key?: string | null;
  connect_timeout?: number | null;
  query_timeout?: number | null;
}

export interface TableInfo {
  name: string;
  type: "table" | "view";
  estimated_rows?: number;
}

export interface NumberFormat {
  precision: number;
}

export interface DatasourceConfig {
  host: string;
  port: number;
  username: string;
  password: string;
  database: string;
  mode: "table" | "sql";
  schema_name: string;
  table_name: string | null;
  selected_fields: string[] | null;
  custom_sql: string | null;
  field_renames: Record<string, string> | null;
  number_formats: Record<string, NumberFormat> | null;
  auto_sync: boolean;
  ssl_mode?: SslMode;
  ssl_root_cert?: string | null;
  ssl_cert?: string | null;
  ssl_key?: string | null;
  connect_timeout?: number | null;
  query_timeout?: number | null;
}
