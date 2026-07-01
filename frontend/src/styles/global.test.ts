import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const css = readFileSync(
  fileURLToPath(new URL("./global.css", import.meta.url)),
  "utf8",
);

describe("global theme", () => {
  it("uses a pure white plugin background", () => {
    expect(css).toContain("--db-bg: #ffffff");
  });
});
