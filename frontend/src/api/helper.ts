import { apiClient } from "./client";
import type { ConnectionInfo, TableInfo } from "../types";
import { mockListTables, mockTestConnection, MOCK_ENABLED } from "./mock";

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
  if (MOCK_ENABLED) return mockTestConnection(conn);
  const { data } = await apiClient.post<ConnectionTestResult>(
    "/api/helper/test_connection",
    conn
  );
  return data;
}

export async function listTables(
  conn: ConnectionInfo & { schema_name: string }
): Promise<TableInfo[]> {
  if (MOCK_ENABLED) return mockListTables(conn);
  const { data } = await apiClient.post<ApiResponse<TableInfo[]>>(
    "/api/helper/tables",
    conn
  );
  return data.data ?? [];
}
