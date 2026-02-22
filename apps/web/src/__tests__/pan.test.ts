import { describe, expect, it } from "vitest";
import { getBaseScale } from "../interaction/coords";
import { applyPan } from "../interaction/pan";
import type { BoundingBox, ViewState } from "../types";

const BOUNDS: BoundingBox = { minX: 0, minY: 0, maxX: 100, maxY: 80 };
const CANVAS_W = 800;
const CANVAS_H = 600;
const FIT_STATE: ViewState = { centerX: 50, centerY: 40, zoom: 1 };

describe("applyPan", () => {
  it("pan by positive deltaX moves center left in board space", () => {
    const result = applyPan(FIT_STATE, 100, 0, CANVAS_W, CANVAS_H, BOUNDS);
    const scale = getBaseScale(BOUNDS, CANVAS_W, CANVAS_H) * FIT_STATE.zoom;

    expect(result.centerX).toBeCloseTo(FIT_STATE.centerX - 100 / scale, 6);
    expect(result.centerY).toBeCloseTo(FIT_STATE.centerY, 6);
    expect(result.zoom).toBe(FIT_STATE.zoom);
  });

  it("pan by positive deltaY moves center up in board space (Y flipped)", () => {
    const result = applyPan(FIT_STATE, 0, 100, CANVAS_W, CANVAS_H, BOUNDS);
    const scale = getBaseScale(BOUNDS, CANVAS_W, CANVAS_H) * FIT_STATE.zoom;

    expect(result.centerX).toBeCloseTo(FIT_STATE.centerX, 6);
    expect(result.centerY).toBeCloseTo(FIT_STATE.centerY + 100 / scale, 6);
    expect(result.zoom).toBe(FIT_STATE.zoom);
  });

  it("pan with null bounds uses fallback scale of 1", () => {
    const result = applyPan(FIT_STATE, 10, 20, CANVAS_W, CANVAS_H, null);

    expect(result.centerX).toBeCloseTo(FIT_STATE.centerX - 10, 6);
    expect(result.centerY).toBeCloseTo(FIT_STATE.centerY + 20, 6);
    expect(result.zoom).toBe(FIT_STATE.zoom);
  });
});
