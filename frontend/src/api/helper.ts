import { apiClient } from "./client";
import type {
  ColumnInfo,
  ConnectionInfo,
  SQLPreviewResult,
  TableInfo,
} from "../types";

interface ApiResponse<T> {
  success: boolean;
  message?: string;
  data?: T;
}

export interface ConnectionTestResult {
  success: boolean;
  message: string;
  server_version: string;
  database_size: string;
  table_count: number;
}

export async function testConnection(
  conn: ConnectionInfo
): Promise<ConnectionTestResult> {
  const { data } = await apiClient.post<ConnectionTestResult>(
    "/api/helper/test_connection",
    conn
  );
  return data;
}

export async function listDatabases(
  conn: ConnectionInfo
): Promise<string[]> {
  const { data } = await apiClient.post<ApiResponse<string[]>>(
    "/api/helper/databases",
    conn
  );
  return data.data ?? [];
}

export async function listSchemas(
  conn: ConnectionInfo
): Promise<string[]> {
  const { data } = await apiClient.post<ApiResponse<string[]>>(
    "/api/helper/schemas",
    conn
  );
  return data.data ?? [];
}

export async function listTables(
  conn: ConnectionInfo & { schema_name: string }
): Promise<TableInfo[]> {
  const { data } = await apiClient.post<ApiResponse<TableInfo[]>>(
    "/api/helper/tables",
    conn
  );
  return data.data ?? [];
}

export async function listColumns(
  conn: ConnectionInfo & { schema_name: string; table_name: string }
): Promise<ColumnInfo[]> {
  const { data } = await apiClient.post<ApiResponse<ColumnInfo[]>>(
    "/api/helper/columns",
    conn
  );
  return data.data ?? [];
}

export async function previewSQL(
  conn: ConnectionInfo & { sql: string }
): Promise<ApiResponse<SQLPreviewResult>> {
  const { data } = await apiClient.post<ApiResponse<SQLPreviewResult>>(
    "/api/helper/preview_sql",
    conn
  );
  return data;
}
