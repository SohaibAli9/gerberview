import { DEFAULT_VIEW_STATE } from "../constants";
import type {
  AppError,
  AppState,
  BoundingBox,
  LoadingProgress,
  ParsedLayer,
  Point,
  ViewState,
} from "../types";
import { createComputed, createSignal, type ReadonlySignal, type Signal } from "./signal";

export interface AppStore {
  readonly appState: Signal<AppState>;
  readonly layers: Signal<ParsedLayer[]>;
  readonly viewState: Signal<ViewState>;
  readonly globalOpacity: Signal<number>;
  readonly error: Signal<AppError | null>;
  readonly loadingProgress: Signal<LoadingProgress | null>;
  readonly cursorPosition: Signal<Point | null>;

  readonly visibleLayers: ReadonlySignal<ParsedLayer[]>;
  readonly boardBounds: ReadonlySignal<BoundingBox | null>;
  readonly boardDimensions: ReadonlySignal<{
    width: number;
    height: number;
  } | null>;
  readonly totalWarnings: ReadonlySignal<number>;

  destroy(): void;
}

function computeUnionBounds(layers: ParsedLayer[]): BoundingBox | null {
  if (layers.length === 0) {
    return null;
  }

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const layer of layers) {
    const { bounds } = layer.meta;
    minX = Math.min(minX, bounds.minX);
    minY = Math.min(minY, bounds.minY);
    maxX = Math.max(maxX, bounds.maxX);
    maxY = Math.max(maxY, bounds.maxY);
  }

  return { minX, minY, maxX, maxY };
}

export function createAppStore(): AppStore {
  const appState = createSignal<AppState>("empty");
  const layers = createSignal<ParsedLayer[]>([]);
  const viewState = createSignal<ViewState>({ ...DEFAULT_VIEW_STATE });
  const globalOpacity = createSignal(1);
  const error = createSignal<AppError | null>(null);
  const loadingProgress = createSignal<LoadingProgress | null>(null);
  const cursorPosition = createSignal<Point | null>(null);

  const visibleLayers = createComputed(() => layers.value.filter((l) => l.visible), [layers]);

  const boardBounds = createComputed(() => computeUnionBounds(layers.value), [layers]);

  const boardDimensions = createComputed(() => {
    const bounds = boardBounds.value;
    if (bounds === null) {
      return null;
    }
    return {
      width: bounds.maxX - bounds.minX,
      height: bounds.maxY - bounds.minY,
    };
  }, [boardBounds]);

  const totalWarnings = createComputed(
    () => layers.value.reduce((sum, layer) => sum + layer.meta.warningCount, 0),
    [layers],
  );

  const computedSignals: { destroy(): void }[] = [
    visibleLayers,
    boardBounds,
    boardDimensions,
    totalWarnings,
  ];

  return {
    appState,
    layers,
    viewState,
    globalOpacity,
    error,
    loadingProgress,
    cursorPosition,
    visibleLayers,
    boardBounds,
    boardDimensions,
    totalWarnings,
    destroy() {
      for (const c of computedSignals) {
        c.destroy();
      }
    },
  };
}
