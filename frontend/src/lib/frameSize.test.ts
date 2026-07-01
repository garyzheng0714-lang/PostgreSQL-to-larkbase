import { describe, expect, it } from "vitest";
import {
  clampFrameHeight,
  clampFrameWidth,
  INITIAL_FRAME_HEIGHT,
  INITIAL_FRAME_WIDTH,
} from "./frameSize";

describe("frame sizing", () => {
  it("uses connector protocol initial dimensions that avoid first-paint clipping", () => {
    expect(INITIAL_FRAME_WIDTH).toBe(520);
    expect(INITIAL_FRAME_HEIGHT).toBe(520);
  });

  it("clamps dynamic frame width to the connector protocol range", () => {
    expect(clampFrameWidth(300)).toBe(420);
    expect(clampFrameWidth(620)).toBe(620);
    expect(clampFrameWidth(1000)).toBe(840);
  });

  it("clamps dynamic frame height to the connector protocol range", () => {
    expect(clampFrameHeight(120)).toBe(226);
    expect(clampFrameHeight(520)).toBe(520);
    expect(clampFrameHeight(900)).toBe(606);
  });
});
