import { describe, expect, it } from "vitest";
import { boardToScreen, fitToView, getBaseScale, screenToBoard } from "../interaction/coords";
import type { BoundingBox, ViewState } from "../types";

const BOUNDS: BoundingBox = { minX: 0, minY: 0, maxX: 100, maxY: 80 };
const CANVAS_W = 800;
const CANVAS_H = 600;

describe("getBaseScale", () => {
  it("returns 1 for null bounds", () => {
    expect(getBaseScale(null, CANVAS_W, CANVAS_H)).toBe(1);
  });

  it("returns 1 for zero-size bounds", () => {
    const zero: BoundingBox = { minX: 5, minY: 5, maxX: 5, maxY: 5 };
    expect(getBaseScale(zero, CANVAS_W, CANVAS_H)).toBe(1);
  });

  it("computes correct scale matching renderer formula", () => {
    const padding = 0.05;
    const scaleX = CANVAS_W / (100 * (1 + 2 * padding));
    const scaleY = CANVAS_H / (80 * (1 + 2 * padding));
    const expected = Math.min(scaleX, scaleY);

    expect(getBaseScale(BOUNDS, CANVAS_W, CANVAS_H)).toBeCloseTo(expected, 10);
  });
});

describe("screenToBoard / boardToScreen", () => {
  it("UT-TS-008: round-trip at fit-to-view preserves coordinates", () => {
    const fitState = fitToView(BOUNDS, CANVAS_W, CANVAS_H);
    const screenCenter = { x: CANVAS_W / 2, y: CANVAS_H / 2 };

    const boardPoint = screenToBoard(screenCenter, fitState, CANVAS_W, CANVAS_H, BOUNDS);
    const backToScreen = boardToScreen(boardPoint, fitState, CANVAS_W, CANVAS_H, BOUNDS);

    expect(backToScreen.x).toBeCloseTo(screenCenter.x, 6);
    expect(backToScreen.y).toBeCloseTo(screenCenter.y, 6);
  });

  it("UT-TS-008: canvas center maps to board center at fit-to-view", () => {
    const fitState = fitToView(BOUNDS, CANVAS_W, CANVAS_H);
    const screenCenter = { x: CANVAS_W / 2, y: CANVAS_H / 2 };

    const boardPoint = screenToBoard(screenCenter, fitState, CANVAS_W, CANVAS_H, BOUNDS);

    expect(boardPoint.x).toBeCloseTo(50, 6);
    expect(boardPoint.y).toBeCloseTo(40, 6);
  });

  it("UT-TS-009: zoom centered on cursor preserves cursor board point", () => {
    const fitState = fitToView(BOUNDS, CANVAS_W, CANVAS_H);
    const cursor = { x: 200, y: 100 };

    const boardBefore = screenToBoard(cursor, fitState, CANVAS_W, CANVAS_H, BOUNDS);

    const newZoom = 2;
    const ratio = fitState.zoom / newZoom;
    const zoomedState: ViewState = {
      centerX: boardBefore.x - (boardBefore.x - fitState.centerX) * ratio,
      centerY: boardBefore.y - (boardBefore.y - fitState.centerY) * ratio,
      zoom: newZoom,
    };

    const boardAfter = screenToBoard(cursor, zoomedState, CANVAS_W, CANVAS_H, BOUNDS);

    expect(boardAfter.x).toBeCloseTo(boardBefore.x, 6);
    expect(boardAfter.y).toBeCloseTo(boardBefore.y, 6);
  });

  it("UT-TS-010: pan by (dx, dy) produces correct translation", () => {
    const fitState = fitToView(BOUNDS, CANVAS_W, CANVAS_H);
    const scale = getBaseScale(BOUNDS, CANVAS_W, CANVAS_H) * fitState.zoom;
    const dx = 50;
    const dy = 30;

    const pannedState: ViewState = {
      centerX: fitState.centerX - dx / scale,
      centerY: fitState.centerY + dy / scale,
      zoom: fitState.zoom,
    };

    const screenCenter = { x: CANVAS_W / 2, y: CANVAS_H / 2 };
    const boardPoint = screenToBoard(screenCenter, pannedState, CANVAS_W, CANVAS_H, BOUNDS);

    expect(boardPoint.x).toBeCloseTo(pannedState.centerX, 6);
    expect(boardPoint.y).toBeCloseTo(pannedState.centerY, 6);
  });

  it("returns origin for null bounds in screenToBoard", () => {
    const state: ViewState = { centerX: 50, centerY: 40, zoom: 1 };
    const result = screenToBoard({ x: 100, y: 100 }, state, CANVAS_W, CANVAS_H, null);

    expect(result.x).toBe(0);
    expect(result.y).toBe(0);
  });

  it("returns origin for null bounds in boardToScreen", () => {
    const state: ViewState = { centerX: 50, centerY: 40, zoom: 1 };
    const result = boardToScreen({ x: 10, y: 10 }, state, CANVAS_W, CANVAS_H, null);

    expect(result.x).toBe(0);
    expect(result.y).toBe(0);
  });
});

describe("fitToView", () => {
  it("UT-TS-011: centers board and sets zoom to 1", () => {
    const result = fitToView(BOUNDS, CANVAS_W, CANVAS_H);

    expect(result.centerX).toBeCloseTo(50, 6);
    expect(result.centerY).toBeCloseTo(40, 6);
    expect(result.zoom).toBe(1);
  });

  it("returns default state for degenerate bounds", () => {
    const zero: BoundingBox = { minX: 5, minY: 5, maxX: 5, maxY: 5 };
    const result = fitToView(zero, CANVAS_W, CANVAS_H);

    expect(result.centerX).toBe(0);
    expect(result.centerY).toBe(0);
    expect(result.zoom).toBe(1);
  });
});
