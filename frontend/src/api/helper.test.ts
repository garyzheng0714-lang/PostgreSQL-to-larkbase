import {
  AxiosError,
  type AxiosResponse,
  type InternalAxiosRequestConfig,
} from "axios";
import { describe, expect, it } from "vitest";
import type { ConnectionInfo } from "../types";
import { apiClient } from "./client";
import { listTables, testConnection } from "./helper";

const conn = { host: "db", port: 5432 } as ConnectionInfo;

describe("helper key", () => {
  it("asks the backend for the key once and fetches it again after the hub rotates it", async () => {
    let backendKey = "k1";
    const calls: string[] = [];
    apiClient.defaults.adapter = async (config: InternalAxiosRequestConfig) => {
      const sent = config.headers.get("X-Helper-Api-Key");
      calls.push(`${config.method} ${config.url} ${sent ?? "-"}`);
      const respond = (status: number, data: unknown): AxiosResponse => ({
        status,
        statusText: "",
        headers: {},
        config,
        data,
      });
      if (config.url === "/api/helper/key") return respond(200, { key: backendKey });
      if (!backendKey || sent !== backendKey) {
        throw new AxiosError("unauthorized", "ERR_BAD_REQUEST", config, null, respond(401, ""));
      }
      return respond(200, { success: true, data: [] });
    };

    await testConnection(conn);
    await listTables({ ...conn, schema_name: "public" });
    backendKey = "k2";
    await testConnection(conn);
    expect(calls).toEqual([
      "get /api/helper/key -",
      "post /api/helper/test_connection k1",
      "post /api/helper/tables k1",
      "post /api/helper/test_connection k1",
      "get /api/helper/key -",
      "post /api/helper/test_connection k2",
    ]);

    // 后端没拿到 key（fail-closed）时只重试一次，401 照样交给界面报错
    backendKey = "";
    calls.length = 0;
    await expect(testConnection(conn)).rejects.toMatchObject({ response: { status: 401 } });
    expect(calls).toEqual([
      "post /api/helper/test_connection k2",
      "get /api/helper/key -",
      "post /api/helper/test_connection -",
    ]);
  });
});
