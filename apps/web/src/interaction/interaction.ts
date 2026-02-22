import type { AppStore } from "../core/store";
import type { Renderer } from "../render/renderer";
import type { SceneManager } from "../scene/scene";
import { AppState } from "../types";
import type { BoundingBox, Point, ViewState } from "../types";
import { fitToView } from "./coords";
import { applyPan } from "./pan";
import { setupTouch } from "./touch";
import { applyZoom } from "./zoom";

const PAN_STEP = 10;

function handleResize(canvas: HTMLCanvasElement, renderer: Renderer): void {
  const width = canvas.clientWidth;
  const height = canvas.clientHeight;
  if (width <= 0 || height <= 0) return;
  canvas.width = width;
  canvas.height = height;
  renderer.resize(width, height);
  renderer.markDirty();
}

/** Sync a new ViewState to the store, renderer, and scene bounds. */
export function updateView(
  newState: ViewState,
  store: AppStore,
  renderer: Renderer,
  sceneManager: SceneManager,
): void {
  store.viewState.value = newState;
  renderer.setViewState(newState);
  renderer.setBoardBounds(sceneManager.getBounds());
  renderer.markDirty();
}

function processKey(
  key: string,
  currentState: ViewState,
  canvasWidth: number,
  canvasHeight: number,
  bounds: BoundingBox | null,
): ViewState | null {
  const cx = canvasWidth / 2;
  const cy = canvasHeight / 2;

  switch (key) {
    case "+":
    case "=":
      return applyZoom(currentState, { x: cx, y: cy }, 1, bounds, canvasWidth, canvasHeight);
    case "-":
      return applyZoom(currentState, { x: cx, y: cy }, -1, bounds, canvasWidth, canvasHeight);
    case "0":
      return bounds !== null ? fitToView(bounds, canvasWidth, canvasHeight) : null;
    case "ArrowUp":
      return applyPan(currentState, 0, -PAN_STEP, canvasWidth, canvasHeight, bounds);
    case "ArrowDown":
      return applyPan(currentState, 0, PAN_STEP, canvasWidth, canvasHeight, bounds);
    case "ArrowLeft":
      return applyPan(currentState, -PAN_STEP, 0, canvasWidth, canvasHeight, bounds);
    case "ArrowRight":
      return applyPan(currentState, PAN_STEP, 0, canvasWidth, canvasHeight, bounds);
    default:
      return null;
  }
}

/** Wire mouse, keyboard, and resize events to zoom, pan, and fit-to-view. */
export function setupInteraction(
  canvas: HTMLCanvasElement,
  store: AppStore,
  renderer: Renderer,
  sceneManager: SceneManager,
): void {
  canvas.setAttribute("tabindex", "0");

  let isDragging = false;
  let lastMouseX = 0;
  let lastMouseY = 0;
  let pendingZoomDelta = 0;
  let pendingCursorScreen: Point | null = null;
  let zoomRafId: number | null = null;

  const flushZoom = (): void => {
    zoomRafId = null;
    if (pendingCursorScreen !== null && pendingZoomDelta !== 0) {
      updateView(
        applyZoom(
          store.viewState.value,
          pendingCursorScreen,
          pendingZoomDelta,
          sceneManager.getBounds(),
          canvas.width,
          canvas.height,
        ),
        store,
        renderer,
        sceneManager,
      );
    }
    pendingZoomDelta = 0;
    pendingCursorScreen = null;
  };

  canvas.addEventListener(
    "wheel",
    (e: WheelEvent): void => {
      e.preventDefault();
      if (store.appState.value !== AppState.Rendered) return;
      const direction = Math.sign(e.deltaY);
      if (direction === 0) return;
      pendingZoomDelta += -direction;
      pendingCursorScreen = { x: e.offsetX, y: e.offsetY };
      zoomRafId ??= requestAnimationFrame(flushZoom);
    },
    { passive: false },
  );

  canvas.addEventListener("mousedown", (e: MouseEvent): void => {
    if (e.button !== 0) return;
    if (store.appState.value !== AppState.Rendered) return;
    isDragging = true;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
    canvas.focus();
  });

  window.addEventListener("mousemove", (e: MouseEvent): void => {
    if (!isDragging) return;
    const dx = e.clientX - lastMouseX;
    const dy = e.clientY - lastMouseY;
    lastMouseX = e.clientX;
    lastMouseY = e.clientY;
    updateView(
      applyPan(
        store.viewState.value,
        dx,
        dy,
        canvas.width,
        canvas.height,
        sceneManager.getBounds(),
      ),
      store,
      renderer,
      sceneManager,
    );
  });

  window.addEventListener("mouseup", (): void => {
    isDragging = false;
  });

  canvas.addEventListener("keydown", (e: KeyboardEvent): void => {
    if (store.appState.value !== AppState.Rendered) return;
    if (e.key === "Escape") {
      canvas.blur();
      return;
    }
    const result = processKey(
      e.key,
      store.viewState.value,
      canvas.width,
      canvas.height,
      sceneManager.getBounds(),
    );
    if (result !== null) {
      e.preventDefault();
      updateView(result, store, renderer, sceneManager);
    }
  });

  window.addEventListener("resize", (): void => {
    handleResize(canvas, renderer);
  });

  setupTouch(canvas, store, renderer, sceneManager);
}
