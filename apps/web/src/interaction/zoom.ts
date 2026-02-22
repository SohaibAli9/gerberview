import { DEFAULT_VIEWER_CONFIG } from "../constants";
import type { BoundingBox, Point, ViewState } from "../types";
import { screenToBoard } from "./coords";

/** Apply cursor-centered zoom. The board point under the cursor stays fixed. */
export function applyZoom(
  currentViewState: ViewState,
  cursorScreen: Point,
  zoomDelta: number,
  boardBounds: BoundingBox | null,
  canvasWidth: number,
  canvasHeight: number,
): ViewState {
  const { zoomFactor, minZoom, maxZoom } = DEFAULT_VIEWER_CONFIG;
  const currentZoom = currentViewState.zoom;
  const rawZoom = currentZoom * Math.pow(zoomFactor, zoomDelta);
  const newZoom = Math.min(maxZoom, Math.max(minZoom, rawZoom));

  if (newZoom === currentZoom) {
    return currentViewState;
  }

  const cursorBoard = screenToBoard(
    cursorScreen,
    currentViewState,
    canvasWidth,
    canvasHeight,
    boardBounds,
  );

  const ratio = currentZoom / newZoom;

  return {
    centerX: cursorBoard.x - (cursorBoard.x - currentViewState.centerX) * ratio,
    centerY: cursorBoard.y - (cursorBoard.y - currentViewState.centerY) * ratio,
    zoom: newZoom,
  };
}
