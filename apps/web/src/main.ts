import "./styles/main.css";
import type { AppStore } from "./core/store";
import { createAppStore } from "./core/store";
import { WorkerClient } from "./engine/worker-client";
import { fitToView, screenToBoard } from "./interaction/coords";
import { setupInteraction } from "./interaction/interaction";
import { applyZoom } from "./interaction/zoom";
import { Renderer } from "./render/renderer";
import { SceneManager } from "./scene/scene";
import { AppState, ErrorCode } from "./types";
import type { ViewState } from "./types";
import { setupUI } from "./ui/ui";
import { setupUploadZone } from "./ui/upload-zone";

function wireFitOnRender(
  canvasEl: HTMLCanvasElement,
  store: AppStore,
  renderer: Renderer,
  sceneManager: SceneManager,
): void {
  store.appState.subscribe((state) => {
    if (state === AppState.Rendered) {
      const bounds = sceneManager.getBounds();
      if (bounds !== null) {
        const viewState = fitToView(bounds, canvasEl.width, canvasEl.height);
        store.viewState.value = viewState;
        renderer.setViewState(viewState);
        renderer.setBoardBounds(bounds);
        renderer.markDirty();
      }
    }
  });
}

function wireCursorTracking(
  canvasEl: HTMLCanvasElement,
  store: AppStore,
  sceneManager: SceneManager,
): void {
  canvasEl.addEventListener("mousemove", (e: MouseEvent) => {
    if (store.appState.value !== AppState.Rendered) return;
    const boardPoint = screenToBoard(
      { x: e.offsetX, y: e.offsetY },
      store.viewState.value,
      canvasEl.width,
      canvasEl.height,
      sceneManager.getBounds(),
    );
    store.cursorPosition.value = boardPoint;
  });

  canvasEl.addEventListener("mouseleave", () => {
    store.cursorPosition.value = null;
  });
}

function wireLoadingOverlay(store: AppStore): void {
  const loadingOverlay = document.getElementById("loading-overlay");
  const loadingText = document.getElementById("loading-text");

  if (loadingOverlay === null) return;

  store.loadingProgress.subscribe((progress) => {
    if (progress !== null) {
      if (loadingText !== null) {
        loadingText.textContent = progress.label;
      }
      loadingOverlay.classList.remove("hidden");
    } else {
      loadingOverlay.classList.add("hidden");
    }
  });
}

function wireViewControls(
  canvasEl: HTMLCanvasElement,
  store: AppStore,
  renderer: Renderer,
  sceneManager: SceneManager,
): void {
  const btnFit = document.getElementById("btn-fit");
  const btnZoomIn = document.getElementById("btn-zoom-in");
  const btnZoomOut = document.getElementById("btn-zoom-out");

  const applyView = (viewState: ViewState): void => {
    store.viewState.value = viewState;
    renderer.setViewState(viewState);
    renderer.setBoardBounds(sceneManager.getBounds());
    renderer.markDirty();
  };

  if (btnFit !== null) {
    btnFit.addEventListener("click", () => {
      const bounds = sceneManager.getBounds();
      if (bounds !== null) {
        applyView(fitToView(bounds, canvasEl.width, canvasEl.height));
      }
    });
  }

  if (btnZoomIn !== null) {
    btnZoomIn.addEventListener("click", () => {
      const cx = canvasEl.width / 2;
      const cy = canvasEl.height / 2;
      applyView(
        applyZoom(
          store.viewState.value,
          { x: cx, y: cy },
          1,
          sceneManager.getBounds(),
          canvasEl.width,
          canvasEl.height,
        ),
      );
    });
  }

  if (btnZoomOut !== null) {
    btnZoomOut.addEventListener("click", () => {
      const cx = canvasEl.width / 2;
      const cy = canvasEl.height / 2;
      applyView(
        applyZoom(
          store.viewState.value,
          { x: cx, y: cy },
          -1,
          sceneManager.getBounds(),
          canvasEl.width,
          canvasEl.height,
        ),
      );
    });
  }
}

async function main(): Promise<void> {
  const store = createAppStore();
  const workerClient = new WorkerClient();

  const canvasEl = document.getElementById("canvas");
  if (!(canvasEl instanceof HTMLCanvasElement)) {
    return;
  }

  canvasEl.width = canvasEl.clientWidth;
  canvasEl.height = canvasEl.clientHeight;

  const gl = canvasEl.getContext("webgl");
  if (gl === null) {
    store.error.value = {
      code: ErrorCode.WebGLUnavailable,
      message: "WebGL is not available. Please use a modern browser with WebGL support.",
    };
    store.appState.value = AppState.Error;
    return;
  }

  const sceneManager = new SceneManager(gl);
  const renderer = new Renderer(gl, sceneManager, canvasEl);

  setupInteraction(canvasEl, store, renderer, sceneManager);

  const uploadZoneEl = document.getElementById("upload-zone");
  if (uploadZoneEl !== null) {
    setupUploadZone(uploadZoneEl, store, workerClient, sceneManager, renderer);
  }

  setupUI(store, renderer);

  wireFitOnRender(canvasEl, store, renderer, sceneManager);
  wireCursorTracking(canvasEl, store, sceneManager);
  wireLoadingOverlay(store);
  wireViewControls(canvasEl, store, renderer, sceneManager);

  await workerClient.waitForReady();
}

void main();
