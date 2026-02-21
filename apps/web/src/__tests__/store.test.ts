import { describe, expect, it } from "vitest";
import { DEFAULT_VIEW_STATE } from "../constants";
import { createAppStore } from "../core/store";
import type { ParsedLayer } from "../types";

function makeMockLayer(overrides: Partial<ParsedLayer> = {}): ParsedLayer {
  return {
    id: "layer-1",
    fileName: "test.gbr",
    layerType: "top_copper",
    color: { r: 1, g: 0, b: 0, a: 1 },
    meta: {
      bounds: { minX: 0, minY: 0, maxX: 10, maxY: 10 },
      vertexCount: 4,
      indexCount: 6,
      commandCount: 1,
      warningCount: 0,
      warnings: [],
    },
    positionBuffer: new Float32Array(0),
    indexBuffer: new Uint32Array(0),
    visible: true,
    opacity: 1,
    ...overrides,
  };
}

describe("AppStore", () => {
  it("returns all expected signals", () => {
    const store = createAppStore();

    expect(store.appState).toBeDefined();
    expect(store.layers).toBeDefined();
    expect(store.viewState).toBeDefined();
    expect(store.globalOpacity).toBeDefined();
    expect(store.error).toBeDefined();
    expect(store.loadingProgress).toBeDefined();
    expect(store.cursorPosition).toBeDefined();
    expect(store.visibleLayers).toBeDefined();
    expect(store.boardBounds).toBeDefined();
    expect(store.boardDimensions).toBeDefined();
    expect(store.totalWarnings).toBeDefined();

    store.destroy();
  });

  it("defaults appState to 'empty'", () => {
    const store = createAppStore();
    expect(store.appState.value).toBe("empty");
    store.destroy();
  });

  it("defaults layers to empty array", () => {
    const store = createAppStore();
    expect(store.layers.value).toEqual([]);
    store.destroy();
  });

  it("defaults viewState to DEFAULT_VIEW_STATE values", () => {
    const store = createAppStore();
    expect(store.viewState.value).toEqual(DEFAULT_VIEW_STATE);
    store.destroy();
  });

  it("filters visibleLayers when layer visibility changes", () => {
    const store = createAppStore();
    const visible = makeMockLayer({ id: "v", visible: true });
    const hidden = makeMockLayer({ id: "h", visible: false });

    store.layers.value = [visible, hidden];

    expect(store.visibleLayers.value).toHaveLength(1);
    expect(store.visibleLayers.value[0]?.id).toBe("v");

    store.destroy();
  });

  it("returns null boardBounds when no layers", () => {
    const store = createAppStore();
    expect(store.boardBounds.value).toBeNull();
    expect(store.boardDimensions.value).toBeNull();
    store.destroy();
  });

  it("sums totalWarnings across layers", () => {
    const store = createAppStore();
    const l1 = makeMockLayer({
      id: "a",
      meta: {
        bounds: { minX: 0, minY: 0, maxX: 1, maxY: 1 },
        vertexCount: 1,
        indexCount: 1,
        commandCount: 1,
        warningCount: 3,
        warnings: ["w1", "w2", "w3"],
      },
    });
    const l2 = makeMockLayer({
      id: "b",
      meta: {
        bounds: { minX: 0, minY: 0, maxX: 1, maxY: 1 },
        vertexCount: 1,
        indexCount: 1,
        commandCount: 1,
        warningCount: 2,
        warnings: ["w4", "w5"],
      },
    });

    store.layers.value = [l1, l2];
    expect(store.totalWarnings.value).toBe(5);

    store.destroy();
  });
});
