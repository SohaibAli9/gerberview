import type { AppStore } from "../core/store";
import type { ParsedLayer, Point } from "../types";

function formatParseStats(layers: readonly ParsedLayer[], warnings: number): string {
  const shapeCount = layers.reduce((sum, layer) => sum + layer.meta.commandCount, 0);
  return `Parsed ${String(layers.length)} layers \u00b7 ${String(shapeCount)} shapes \u00b7 ${String(warnings)} warnings`;
}

function formatCursorPosition(point: Point): string {
  return `X: ${point.x.toFixed(1)}mm  Y: ${point.y.toFixed(1)}mm`;
}

function updateStatusText(statusText: HTMLElement, store: AppStore): void {
  const progress = store.loadingProgress.value;
  if (progress !== null) {
    statusText.textContent = progress.label;
    return;
  }

  const layers = store.layers.value;
  if (layers.length > 0) {
    statusText.textContent = formatParseStats(layers, store.totalWarnings.value);
    return;
  }

  statusText.textContent = "Ready";
}

/**
 * Initialize the status bar with parse stats, loading progress,
 * and cursor coordinate display.
 */
export function setupStatusBar(container: HTMLElement, store: AppStore): void {
  const statusText = container.querySelector<HTMLElement>("#status-text");
  const versionEl = container.querySelector<HTMLElement>("#version");

  const cursorCoords = document.createElement("span");
  cursorCoords.id = "cursor-coords";
  cursorCoords.className = "text-gray-400";
  if (versionEl !== null) {
    container.insertBefore(cursorCoords, versionEl);
  } else {
    container.appendChild(cursorCoords);
  }

  if (statusText !== null) {
    store.loadingProgress.subscribe(() => {
      updateStatusText(statusText, store);
    });

    store.layers.subscribe(() => {
      updateStatusText(statusText, store);
    });

    store.totalWarnings.subscribe(() => {
      updateStatusText(statusText, store);
    });
  }

  store.cursorPosition.subscribe((pos: Point | null) => {
    if (pos !== null) {
      cursorCoords.textContent = formatCursorPosition(pos);
    } else {
      cursorCoords.textContent = "";
    }
  });
}
