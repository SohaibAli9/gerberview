import type { AppStore } from "../core/store";
import type { Renderer } from "../render/renderer";
import type { LayerColor, LayerType, ParsedLayer } from "../types";
import { AppState } from "../types";

const LAYER_DISPLAY_NAMES: Readonly<Record<LayerType, string>> = {
  top_copper: "Top Copper",
  bottom_copper: "Bottom Copper",
  top_solder_mask: "Top Solder Mask",
  bottom_solder_mask: "Bottom Solder Mask",
  top_silkscreen: "Top Silkscreen",
  bottom_silkscreen: "Bottom Silkscreen",
  top_paste: "Top Paste",
  bottom_paste: "Bottom Paste",
  board_outline: "Board Outline",
  drill: "Drill",
  inner_copper: "Inner Copper",
  unknown: "Unknown",
};

function layerColorToCSS(color: LayerColor): string {
  return `rgba(${String(Math.round(color.r * 255))}, ${String(Math.round(color.g * 255))}, ${String(Math.round(color.b * 255))}, ${String(color.a)})`;
}

function formatLayerName(layer: ParsedLayer): string {
  const displayName = LAYER_DISPLAY_NAMES[layer.layerType];
  if (displayName !== "Unknown") {
    return displayName;
  }
  return layer.fileName;
}

function createLayerItem(
  layer: ParsedLayer,
  store: AppStore,
  renderer: Renderer,
): HTMLLabelElement {
  const label = document.createElement("label");
  label.className = "flex items-center gap-2 py-1 cursor-pointer text-sm text-gray-300";

  const checkbox = document.createElement("input");
  checkbox.type = "checkbox";
  checkbox.checked = layer.visible;
  checkbox.className = "accent-blue-500 shrink-0";
  checkbox.addEventListener("change", () => {
    layer.visible = checkbox.checked;
    store.layers.value = [...store.layers.value];
    renderer.markDirty();
  });

  const swatch = document.createElement("span");
  swatch.className = "inline-block w-3 h-3 rounded-sm shrink-0";
  swatch.style.backgroundColor = layerColorToCSS(layer.color);

  const nameSpan = document.createElement("span");
  nameSpan.className = "truncate";
  nameSpan.textContent = formatLayerName(layer);

  label.append(checkbox, swatch, nameSpan);
  return label;
}

function buildLayerList(
  fieldset: HTMLFieldSetElement,
  layers: readonly ParsedLayer[],
  store: AppStore,
  renderer: Renderer,
): void {
  const legend = fieldset.querySelector("legend");

  while (fieldset.lastChild !== null && fieldset.lastChild !== legend) {
    fieldset.removeChild(fieldset.lastChild);
  }

  for (const layer of layers) {
    fieldset.appendChild(createLayerItem(layer, store, renderer));
  }
}

function createOpacityControl(container: HTMLElement, store: AppStore, renderer: Renderer): void {
  const wrapper = document.createElement("div");
  wrapper.className = "mt-3 pt-3 border-t border-board-border";

  const label = document.createElement("label");
  label.className = "text-xs text-gray-500 uppercase tracking-wider block mb-1";
  label.textContent = "Opacity";

  const slider = document.createElement("input");
  slider.type = "range";
  slider.min = "0";
  slider.max = "1";
  slider.step = "0.01";
  slider.value = String(store.globalOpacity.value);
  slider.className = "w-full accent-blue-500";
  slider.id = "opacity-slider";
  slider.setAttribute("aria-label", "Global layer opacity");
  label.htmlFor = "opacity-slider";

  slider.addEventListener("input", () => {
    const value = parseFloat(slider.value);
    store.globalOpacity.value = value;
    renderer.setGlobalOpacity(value);
    renderer.markDirty();
  });

  wrapper.append(label, slider);
  container.appendChild(wrapper);
}

function createBoardDimensionsDisplay(container: HTMLElement, store: AppStore): void {
  const el = document.createElement("p");
  el.className = "text-xs text-gray-400 mt-2 hidden";

  store.boardDimensions.subscribe((dims) => {
    if (dims !== null) {
      el.textContent = `${dims.width.toFixed(1)} \u00d7 ${dims.height.toFixed(1)} mm`;
      el.classList.remove("hidden");
    } else {
      el.textContent = "";
      el.classList.add("hidden");
    }
  });

  container.appendChild(el);
}

/**
 * Initialize the layer panel with per-layer checkboxes, color swatches,
 * a global opacity slider, and board dimensions display.
 */
export function setupLayerPanel(container: HTMLElement, store: AppStore, renderer: Renderer): void {
  container.innerHTML = "";

  const fieldset = document.createElement("fieldset");
  fieldset.className = "border-none p-0 m-0";
  const legend = document.createElement("legend");
  legend.className = "text-xs text-gray-500 uppercase tracking-wider mb-2";
  legend.textContent = "Layers";
  fieldset.appendChild(legend);
  container.appendChild(fieldset);

  store.layers.subscribe((layers) => {
    buildLayerList(fieldset, layers, store, renderer);
  });

  createOpacityControl(container, store, renderer);
  createBoardDimensionsDisplay(container, store);

  store.appState.subscribe((state) => {
    container.classList.toggle("hidden", state !== AppState.Rendered);
  });
}
