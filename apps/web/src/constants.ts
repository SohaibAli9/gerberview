import type { LayerColor, LayerType, ViewerConfig, ViewState } from "./types";

export const LAYER_COLORS: Readonly<Record<LayerType, LayerColor>> = Object.freeze({
  top_copper: { r: 0.8, g: 0.2, b: 0.2, a: 0.9 },
  bottom_copper: { r: 0.2, g: 0.2, b: 0.8, a: 0.9 },
  top_solder_mask: { r: 0.1, g: 0.5, b: 0.1, a: 0.5 },
  bottom_solder_mask: { r: 0.1, g: 0.5, b: 0.1, a: 0.5 },
  top_silkscreen: { r: 0.9, g: 0.9, b: 0.9, a: 0.9 },
  bottom_silkscreen: { r: 0.7, g: 0.7, b: 0.9, a: 0.9 },
  top_paste: { r: 0.8, g: 0.8, b: 0.8, a: 0.5 },
  bottom_paste: { r: 0.8, g: 0.8, b: 0.8, a: 0.5 },
  board_outline: { r: 0.6, g: 0.6, b: 0.6, a: 1.0 },
  drill: { r: 0.9, g: 0.9, b: 0.2, a: 1.0 },
  inner_copper: { r: 0.6, g: 0.4, b: 0.8, a: 0.9 },
  unknown: { r: 0.5, g: 0.5, b: 0.5, a: 0.6 },
});

export const LAYER_Z_ORDER: Readonly<Record<LayerType, number>> = Object.freeze({
  board_outline: 0,
  bottom_paste: 1,
  bottom_solder_mask: 2,
  bottom_silkscreen: 3,
  bottom_copper: 4,
  inner_copper: 5,
  top_copper: 6,
  top_silkscreen: 7,
  top_solder_mask: 8,
  top_paste: 9,
  drill: 10,
  unknown: 11,
});

export const DEFAULT_VIEWER_CONFIG: Readonly<ViewerConfig> = Object.freeze({
  minZoom: 0.001,
  maxZoom: 10_000,
  zoomFactor: 1.5,
  fitPadding: 0.05,
  backgroundColor: [0.102, 0.102, 0.102, 1.0] as const,
});

export const DEFAULT_VIEW_STATE: Readonly<ViewState> = Object.freeze({
  centerX: 0,
  centerY: 0,
  zoom: 1,
});
