interface HelperHttpError {
  code?: string;
  response?: {
    status?: number;
    data?: unknown;
  };
}

const HELPER_AUTH_MESSAGE =
  "后端辅助接口鉴权失败，请检查 HELPER_API_KEY 与前端 VITE_HELPER_API_KEY 是否一致";

export function getHelperErrorMessage(
  error: unknown,
  fallback: string,
): string {
  const err = error as HelperHttpError;
  const status = err.response?.status;

  if (status === 401) return HELPER_AUTH_MESSAGE;

  const responseMessage = extractResponseMessage(err.response?.data);
  if (responseMessage) return responseMessage;

  if (err.code === "ECONNABORTED" || err.code === "ETIMEDOUT") {
    return "后端服务响应超时，请稍后重试";
  }

  if (status) {
    return `后端服务返回异常（HTTP ${status}），请稍后重试`;
  }

  return fallback;
}

function extractResponseMessage(data: unknown): string | null {
  if (typeof data === "string") {
    const message = data.trim();
    return message || null;
  }
  if (!isRecord(data)) return null;

  for (const key of ["message", "detail", "msg", "error"]) {
    const value = data[key];
    if (typeof value === "string" && value.trim()) return value.trim();
  }

  return null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
