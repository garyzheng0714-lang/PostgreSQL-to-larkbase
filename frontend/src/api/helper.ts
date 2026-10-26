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

// helper key 由 FBIF 中台下发给后端，不在构建时写进前端：第一次调 helper 前向后端要，之后复用。
// 中台换了 key 会重建后端，手里的旧 key 遇到 401 就重新要一次再试。
let helperKey: Promise<string> | null = null;

function getHelperKey(): Promise<string> {
  helperKey ??= apiClient
    .get<{ key?: string }>("/api/helper/key")
    .then(({ data }) => data.key ?? "")
    .catch((error) => {
      helperKey = null;
      throw error;
    });
  return helperKey;
}

async function helperPost<T>(path: string, body: unknown, retried = false): Promise<T> {
  const key = await getHelperKey();
  try {
    const { data } = await apiClient.post<T>(path, body, {
      headers: key ? { "X-Helper-Api-Key": key } : {},
    });
    return data;
  } catch (error) {
    const status = (error as { response?: { status?: number } }).response?.status;
    if (status !== 401 || retried) throw error;
    helperKey = null;
    return helperPost<T>(path, body, true);
  }
}

export async function testConnection(
  conn: ConnectionInfo
): Promise<ConnectionTestResult> {
  if (MOCK_ENABLED) return mockTestConnection(conn);
  return helperPost<ConnectionTestResult>("/api/helper/test_connection", conn);
}

export async function listTables(
  conn: ConnectionInfo & { schema_name: string }
): Promise<TableInfo[]> {
  if (MOCK_ENABLED) return mockListTables(conn);
  const data = await helperPost<ApiResponse<TableInfo[]>>("/api/helper/tables", conn);
  return data.data ?? [];
}
