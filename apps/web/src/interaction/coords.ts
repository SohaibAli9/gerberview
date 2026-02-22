import { DEFAULT_VIEWER_CONFIG } from "../constants";
import type { BoundingBox, Point, ViewState } from "../types";

/** Compute the base scale factor that fits the board in the canvas at zoom=1. */
export function getBaseScale(
  boardBounds: BoundingBox | null,
  canvasWidth: number,
  canvasHeight: number,
): number {
  if (boardBounds === null || canvasWidth <= 0 || canvasHeight <= 0) {
    return 1;
  }

  const boardWidth = boardBounds.maxX - boardBounds.minX;
  const boardHeight = boardBounds.maxY - boardBounds.minY;

  if (boardWidth <= 0 || boardHeight <= 0) {
    return 1;
  }

  const padding = DEFAULT_VIEWER_CONFIG.fitPadding;
  const scaleX = canvasWidth / (boardWidth * (1 + 2 * padding));
  const scaleY = canvasHeight / (boardHeight * (1 + 2 * padding));

  return Math.min(scaleX, scaleY);
}

/** Convert screen pixel coordinates to board coordinates. */
export function screenToBoard(
  screenPoint: Point,
  viewState: ViewState,
  canvasWidth: number,
  canvasHeight: number,
  boardBounds: BoundingBox | null,
): Point {
  if (boardBounds === null) {
    return { x: 0, y: 0 };
  }

  const scale = getBaseScale(boardBounds, canvasWidth, canvasHeight) * viewState.zoom;

  if (scale <= 0) {
    return { x: 0, y: 0 };
  }

  return {
    x: (screenPoint.x - canvasWidth / 2) / scale + viewState.centerX,
    y: (canvasHeight / 2 - screenPoint.y) / scale + viewState.centerY,
  };
}

/** Convert board coordinates to screen pixel coordinates. */
export function boardToScreen(
  boardPoint: Point,
  viewState: ViewState,
  canvasWidth: number,
  canvasHeight: number,
  boardBounds: BoundingBox | null,
): Point {
  if (boardBounds === null) {
    return { x: 0, y: 0 };
  }

  const scale = getBaseScale(boardBounds, canvasWidth, canvasHeight) * viewState.zoom;

  return {
    x: (boardPoint.x - viewState.centerX) * scale + canvasWidth / 2,
    y: canvasHeight / 2 - (boardPoint.y - viewState.centerY) * scale,
  };
}

/** Compute the ViewState that fits the given bounds centered in the canvas. */
export function fitToView(
  bounds: BoundingBox,
  canvasWidth: number,
  canvasHeight: number,
): ViewState {
  const boardWidth = bounds.maxX - bounds.minX;
  const boardHeight = bounds.maxY - bounds.minY;

  if (boardWidth <= 0 || boardHeight <= 0 || canvasWidth <= 0 || canvasHeight <= 0) {
    return { centerX: 0, centerY: 0, zoom: 1 };
  }

  return {
    centerX: (bounds.minX + bounds.maxX) / 2,
    centerY: (bounds.minY + bounds.maxY) / 2,
    zoom: 1,
  };
}
