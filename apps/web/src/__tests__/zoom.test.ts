import { describe, expect, it } from "vitest";
import { DEFAULT_VIEWER_CONFIG } from "../constants";
import { screenToBoard } from "../interaction/coords";
import { applyZoom } from "../interaction/zoom";
import type { BoundingBox, ViewState } from "../types";

const BOUNDS: BoundingBox = { minX: 0, minY: 0, maxX: 100, maxY: 80 };
const CANVAS_W = 800;
const CANVAS_H = 600;
const FIT_STATE: ViewState = { centerX: 50, centerY: 40, zoom: 1 };

describe("applyZoom", () => {
  it("zoom in increases zoom level by zoomFactor", () => {
    const cursor = { x: CANVAS_W / 2, y: CANVAS_H / 2 };
    const result = applyZoom(FIT_STATE, cursor, 1, BOUNDS, CANVAS_W, CANVAS_H);

    expect(result.zoom).toBeCloseTo(DEFAULT_VIEWER_CONFIG.zoomFactor, 10);
  });

  it("zoom out decreases zoom level by 1/zoomFactor", () => {
    const cursor = { x: CANVAS_W / 2, y: CANVAS_H / 2 };
    const result = applyZoom(FIT_STATE, cursor, -1, BOUNDS, CANVAS_W, CANVAS_H);

    expect(result.zoom).toBeCloseTo(1 / DEFAULT_VIEWER_CONFIG.zoomFactor, 10);
  });

  it("clamps at maxZoom when zooming in at limit", () => {
    const cursor = { x: CANVAS_W / 2, y: CANVAS_H / 2 };
    const atMax: ViewState = { centerX: 50, centerY: 40, zoom: DEFAULT_VIEWER_CONFIG.maxZoom };
    const result = applyZoom(atMax, cursor, 1, BOUNDS, CANVAS_W, CANVAS_H);

    expect(result).toBe(atMax);
    expect(result.zoom).toBe(DEFAULT_VIEWER_CONFIG.maxZoom);
  });

  it("clamps at minZoom when zooming out at limit", () => {
    const cursor = { x: CANVAS_W / 2, y: CANVAS_H / 2 };
    const atMin: ViewState = { centerX: 50, centerY: 40, zoom: DEFAULT_VIEWER_CONFIG.minZoom };
    const result = applyZoom(atMin, cursor, -1, BOUNDS, CANVAS_W, CANVAS_H);

    expect(result).toBe(atMin);
    expect(result.zoom).toBe(DEFAULT_VIEWER_CONFIG.minZoom);
  });

  it("cursor point stays fixed after zoom", () => {
    const cursor = { x: 600, y: 200 };

    const boardBefore = screenToBoard(cursor, FIT_STATE, CANVAS_W, CANVAS_H, BOUNDS);
    const zoomed = applyZoom(FIT_STATE, cursor, 1, BOUNDS, CANVAS_W, CANVAS_H);
    const boardAfter = screenToBoard(cursor, zoomed, CANVAS_W, CANVAS_H, BOUNDS);

    expect(boardAfter.x).toBeCloseTo(boardBefore.x, 6);
    expect(boardAfter.y).toBeCloseTo(boardBefore.y, 6);
  });
});
