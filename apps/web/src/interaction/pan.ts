import type { BoundingBox, ViewState } from "../types";
import { getBaseScale } from "./coords";

/** Apply pan by converting pixel deltas to board-space movement. */
export function applyPan(
  currentViewState: ViewState,
  deltaX: number,
  deltaY: number,
  canvasWidth: number,
  canvasHeight: number,
  boardBounds: BoundingBox | null,
): ViewState {
  const scale = getBaseScale(boardBounds, canvasWidth, canvasHeight) * currentViewState.zoom;

  if (scale <= 0) {
    return currentViewState;
  }

  return {
    centerX: currentViewState.centerX - deltaX / scale,
    centerY: currentViewState.centerY + deltaY / scale,
    zoom: currentViewState.zoom,
  };
}
