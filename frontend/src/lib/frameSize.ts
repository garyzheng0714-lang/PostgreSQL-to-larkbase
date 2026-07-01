export const MIN_FRAME_WIDTH = 420;
export const MAX_FRAME_WIDTH = 840;
export const MIN_FRAME_HEIGHT = 226;
export const MAX_FRAME_HEIGHT = 606;

export const INITIAL_FRAME_WIDTH = 520;
export const INITIAL_FRAME_HEIGHT = 520;

export function clampFrameWidth(width: number): number {
  return clamp(Math.ceil(width || INITIAL_FRAME_WIDTH), MIN_FRAME_WIDTH, MAX_FRAME_WIDTH);
}

export function clampFrameHeight(height: number): number {
  return clamp(
    Math.ceil(height || INITIAL_FRAME_HEIGHT),
    MIN_FRAME_HEIGHT,
    MAX_FRAME_HEIGHT,
  );
}

export function getAdaptiveFrameWidth(): number {
  if (typeof window === "undefined") return INITIAL_FRAME_WIDTH;
  return clampFrameWidth(window.innerWidth || INITIAL_FRAME_WIDTH);
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
