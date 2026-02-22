import type { AppStore } from "../core/store";
import type { Renderer } from "../render/renderer";
import { setupErrorBanner } from "./error-banner";
import { setupLayerPanel } from "./layer-panel";
import { setupStatusBar } from "./status-bar";

/**
 * Wire up all UI components (layer panel, status bar, error banner)
 * to the application store and renderer.
 */
export function setupUI(store: AppStore, renderer: Renderer): void {
  const layerPanelEl = document.getElementById("layer-panel");
  if (layerPanelEl !== null) {
    setupLayerPanel(layerPanelEl, store, renderer);
  }

  const statusBarEl = document.getElementById("status-bar");
  if (statusBarEl !== null) {
    setupStatusBar(statusBarEl, store);
  }

  const errorBannerEl = document.getElementById("error-banner");
  if (errorBannerEl !== null) {
    setupErrorBanner(errorBannerEl, store);
  }
}
