import { DEFAULT_VIEWER_CONFIG } from "../constants";
import type { AppStore } from "../core/store";
import type { Renderer } from "../render/renderer";
import type { SceneManager } from "../scene/scene";
import { AppState } from "../types";
import type { BoundingBox, Point, ViewState } from "../types";
import { screenToBoard } from "./coords";
import { applyPan } from "./pan";

type TouchMode = "idle" | "pending_pan" | "panning" | "pinching";

type ViewUpdateFn = (
  newState: ViewState,
  store: AppStore,
  renderer: Renderer,
  sceneManager: SceneManager,
) => void;

interface TouchState {
  mode: TouchMode;
  lastX: number;
  lastY: number;
  startTime: number;
  lastDistance: number;
}

const PAN_HOLD_DELAY_MS = 50;

function getTouchDistance(t0: Touch, t1: Touch): number {
  return Math.hypot(t1.clientX - t0.clientX, t1.clientY - t0.clientY);
}

function getTouchMidpoint(t0: Touch, t1: Touch, rect: DOMRect): Point {
  return {
    x: (t0.clientX + t1.clientX) / 2 - rect.left,
    y: (t0.clientY + t1.clientY) / 2 - rect.top,
  };
}

function computePinchViewState(
  current: ViewState,
  pinchRatio: number,
  midpointScreen: Point,
  boardBounds: BoundingBox | null,
  canvasWidth: number,
  canvasHeight: number,
): ViewState {
  const { minZoom, maxZoom } = DEFAULT_VIEWER_CONFIG;
  const newZoom = Math.min(maxZoom, Math.max(minZoom, current.zoom * pinchRatio));

  if (newZoom === current.zoom) {
    return current;
  }

  const mid = screenToBoard(midpointScreen, current, canvasWidth, canvasHeight, boardBounds);
  const ratio = current.zoom / newZoom;

  return {
    centerX: mid.x - (mid.x - current.centerX) * ratio,
    centerY: mid.y - (mid.y - current.centerY) * ratio,
    zoom: newZoom,
  };
}

function resetTouchState(state: TouchState): void {
  state.mode = "idle";
  state.lastX = 0;
  state.lastY = 0;
  state.startTime = 0;
  state.lastDistance = 0;
}

function handleTouchStart(
  e: TouchEvent,
  state: TouchState,
  canvas: HTMLCanvasElement,
  store: AppStore,
): void {
  if (store.appState.value !== AppState.Rendered) return;

  const touchCount = e.touches.length;

  if (touchCount >= 2) {
    const t0 = e.touches[0];
    const t1 = e.touches[1];
    if (t0 === undefined || t1 === undefined) return;
    state.mode = "pinching";
    state.lastDistance = getTouchDistance(t0, t1);
    e.preventDefault();
    return;
  }

  if (touchCount === 1) {
    const t0 = e.touches[0];
    if (t0 === undefined) return;
    const rect = canvas.getBoundingClientRect();
    state.mode = "pending_pan";
    state.lastX = t0.clientX - rect.left;
    state.lastY = t0.clientY - rect.top;
    state.startTime = Date.now();
  }
}

function handlePinchMove(
  e: TouchEvent,
  state: TouchState,
  canvas: HTMLCanvasElement,
  store: AppStore,
  renderer: Renderer,
  sceneManager: SceneManager,
  onViewUpdate: ViewUpdateFn,
): void {
  e.preventDefault();

  const t0 = e.touches[0];
  const t1 = e.touches[1];
  if (t0 === undefined || t1 === undefined) return;

  const newDistance = getTouchDistance(t0, t1);

  if (state.lastDistance <= 0) {
    state.lastDistance = newDistance;
    return;
  }

  const ratio = newDistance / state.lastDistance;
  const rect = canvas.getBoundingClientRect();
  const midpoint = getTouchMidpoint(t0, t1, rect);

  const newState = computePinchViewState(
    store.viewState.value,
    ratio,
    midpoint,
    sceneManager.getBounds(),
    canvas.width,
    canvas.height,
  );

  onViewUpdate(newState, store, renderer, sceneManager);
  state.lastDistance = newDistance;
}

function handlePanMove(
  e: TouchEvent,
  state: TouchState,
  canvas: HTMLCanvasElement,
  store: AppStore,
  renderer: Renderer,
  sceneManager: SceneManager,
  onViewUpdate: ViewUpdateFn,
): void {
  const elapsed = Date.now() - state.startTime;
  if (elapsed < PAN_HOLD_DELAY_MS) return;

  e.preventDefault();

  const t0 = e.touches[0];
  if (t0 === undefined) return;

  const rect = canvas.getBoundingClientRect();
  const currentX = t0.clientX - rect.left;
  const currentY = t0.clientY - rect.top;

  if (state.mode === "pending_pan") {
    state.mode = "panning";
    state.lastX = currentX;
    state.lastY = currentY;
    return;
  }

  const dx = currentX - state.lastX;
  const dy = currentY - state.lastY;
  state.lastX = currentX;
  state.lastY = currentY;

  const newState = applyPan(
    store.viewState.value,
    dx,
    dy,
    canvas.width,
    canvas.height,
    sceneManager.getBounds(),
  );

  onViewUpdate(newState, store, renderer, sceneManager);
}

function handleTouchMove(
  e: TouchEvent,
  state: TouchState,
  canvas: HTMLCanvasElement,
  store: AppStore,
  renderer: Renderer,
  sceneManager: SceneManager,
  onViewUpdate: ViewUpdateFn,
): void {
  if (store.appState.value !== AppState.Rendered) return;

  const touchCount = e.touches.length;

  if (state.mode === "pinching" && touchCount >= 2) {
    handlePinchMove(e, state, canvas, store, renderer, sceneManager, onViewUpdate);
    return;
  }

  if (touchCount === 1 && (state.mode === "pending_pan" || state.mode === "panning")) {
    handlePanMove(e, state, canvas, store, renderer, sceneManager, onViewUpdate);
  }
}

function handleTouchEnd(e: TouchEvent, state: TouchState): void {
  const remaining = e.touches.length;

  if (remaining === 0) {
    resetTouchState(state);
    return;
  }

  if (state.mode === "pinching" && remaining < 2) {
    resetTouchState(state);
  }
}

/** Wire touch events for pinch-to-zoom and single-finger drag-to-pan. */
export function setupTouch(
  canvas: HTMLCanvasElement,
  store: AppStore,
  renderer: Renderer,
  sceneManager: SceneManager,
  onViewUpdate: ViewUpdateFn,
): void {
  const state: TouchState = {
    mode: "idle",
    lastX: 0,
    lastY: 0,
    startTime: 0,
    lastDistance: 0,
  };

  canvas.addEventListener(
    "touchstart",
    (e: TouchEvent): void => {
      handleTouchStart(e, state, canvas, store);
    },
    { passive: false },
  );

  canvas.addEventListener(
    "touchmove",
    (e: TouchEvent): void => {
      handleTouchMove(e, state, canvas, store, renderer, sceneManager, onViewUpdate);
    },
    { passive: false },
  );

  canvas.addEventListener("touchend", (e: TouchEvent): void => {
    handleTouchEnd(e, state);
  });

  canvas.addEventListener("touchcancel", (): void => {
    resetTouchState(state);
  });
}
