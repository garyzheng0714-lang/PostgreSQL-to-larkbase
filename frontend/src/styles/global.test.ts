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

  it("uses Feishu-native primary color and compact control tokens", () => {
    expect(css).toContain("--db-accent: #1456f0");
    expect(css).toContain("--db-field-h: 32px");
    expect(css).toContain("--db-radius: 6px");
    expect(css).toContain("--db-radius-lg: 8px");
  });

  it("keeps the sidebar configuration surface flat instead of card-in-modal", () => {
    const cardRule = css.match(/\.db-card\s*\{(?<body>[^}]+)\}/)?.groups?.body ?? "";

    expect(cardRule).toContain("border: none");
    expect(cardRule).toContain("box-shadow: none");
  });
});
