import { describe, expect, it } from "vitest";
import { getHelperErrorMessage } from "./errors";

describe("getHelperErrorMessage", () => {
  it("explains helper auth failures instead of showing a network fallback", () => {
    const message = getHelperErrorMessage(
      {
        response: {
          status: 401,
          data: "Invalid or missing helper API key",
        },
      },
      "网络错误，无法连接后端服务",
    );

    expect(message).toContain("后端辅助接口鉴权失败");
    expect(message).not.toBe("网络错误，无法连接后端服务");
  });

  it("keeps backend messages when the helper returns one", () => {
    expect(
      getHelperErrorMessage(
        {
          response: {
            status: 400,
            data: { message: "数据库地址无效" },
          },
        },
        "网络错误，无法连接后端服务",
      ),
    ).toBe("数据库地址无效");
  });

  it("has a specific timeout message", () => {
    expect(
      getHelperErrorMessage(
        { code: "ECONNABORTED" },
        "网络错误，无法连接后端服务",
      ),
    ).toContain("响应超时");
  });
});
