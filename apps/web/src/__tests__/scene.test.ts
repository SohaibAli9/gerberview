import { describe, expect, it, vi } from "vitest";
import { SceneManager } from "../scene/scene";
import type { BoundingBox, LayerColor, LayerMeta, ParsedLayer } from "../types";
import { LayerType } from "../types";

const ARRAY_BUFFER = 0x8892;
const ELEMENT_ARRAY_BUFFER = 0x8893;
const STATIC_DRAW = 0x88e4;

interface MockGL extends WebGLRenderingContext {
  createBuffer: ReturnType<typeof vi.fn>;
  bindBuffer: ReturnType<typeof vi.fn>;
  bufferData: ReturnType<typeof vi.fn>;
  deleteBuffer: ReturnType<typeof vi.fn>;
}

function createMockGL(): MockGL {
  let bufferId = 0;
  const createBuffer = vi.fn(() => {
    bufferId += 1;
    return { __bufferId: bufferId } as unknown as WebGLBuffer;
  });
  const bindBuffer = vi.fn();
  const bufferData = vi.fn();
  const deleteBuffer = vi.fn();

  const gl = {
    createBuffer,
    bindBuffer,
    bufferData,
    deleteBuffer,
    ARRAY_BUFFER,
    ELEMENT_ARRAY_BUFFER,
    STATIC_DRAW,
  } as unknown as MockGL;

  return gl;
}

function createParsedLayer(
  id: string,
  layerType: (typeof LayerType)[keyof typeof LayerType],
  bounds: BoundingBox,
  options?: {
    positionBuffer?: Float32Array;
    indexBuffer?: Uint32Array;
    visible?: boolean;
  },
): ParsedLayer {
  const positionBuffer = options?.positionBuffer ?? new Float32Array([0, 0, 1, 0, 0, 1]);
  const indexBuffer = options?.indexBuffer ?? new Uint32Array([0, 1, 2]);
  const meta: LayerMeta = {
    bounds,
    vertexCount: positionBuffer.length / 2,
    indexCount: indexBuffer.length,
    commandCount: 1,
    warningCount: 0,
    warnings: [],
  };
  const color: LayerColor = { r: 0.5, g: 0.5, b: 0.5, a: 1 };
  return {
    id,
    fileName: `${id}.gbr`,
    layerType,
    color,
    meta,
    positionBuffer,
    indexBuffer,
    visible: options?.visible ?? true,
    opacity: 1,
  };
}

describe("SceneManager addLayer", () => {
  it("creates VBOs and inserts node in z-order", () => {
    const gl = createMockGL();
    const manager = new SceneManager(gl);

    const topCopper = createParsedLayer("top", LayerType.TopCopper, {
      minX: 0,
      minY: 0,
      maxX: 10,
      maxY: 10,
    });
    const boardOutline = createParsedLayer("outline", LayerType.BoardOutline, {
      minX: 0,
      minY: 0,
      maxX: 10,
      maxY: 10,
    });
    const drill = createParsedLayer("drill", LayerType.Drill, {
      minX: 0,
      minY: 0,
      maxX: 10,
      maxY: 10,
    });

    manager.addLayer(topCopper);
    manager.addLayer(boardOutline);
    manager.addLayer(drill);

    const visible = manager.getVisibleLayers();
    expect(visible).toHaveLength(3);
    expect(visible[0]?.layerType).toBe(LayerType.BoardOutline);
    expect(visible[1]?.layerType).toBe(LayerType.TopCopper);
    expect(visible[2]?.layerType).toBe(LayerType.Drill);
  });

  it("uploads position and index buffers", () => {
    const gl = createMockGL();
    const manager = new SceneManager(gl);

    const positions = new Float32Array([1, 2, 3, 4, 5, 6]);
    const indices = new Uint32Array([0, 1, 2]);
    const layer = createParsedLayer(
      "layer1",
      LayerType.TopCopper,
      {
        minX: 0,
        minY: 0,
        maxX: 1,
        maxY: 1,
      },
      {
        positionBuffer: positions,
        indexBuffer: indices,
      },
    );

    manager.addLayer(layer);

    expect(gl.bufferData).toHaveBeenCalledWith(ARRAY_BUFFER, positions, STATIC_DRAW);
    expect(gl.bufferData).toHaveBeenCalledWith(ELEMENT_ARRAY_BUFFER, indices, STATIC_DRAW);
  });
});

describe("SceneManager getVisibleLayers", () => {
  it("excludes hidden nodes", () => {
    const gl = createMockGL();
    const manager = new SceneManager(gl);

    const layer1 = createParsedLayer("layer1", LayerType.TopCopper, {
      minX: 0,
      minY: 0,
      maxX: 10,
      maxY: 10,
    });
    const layer2 = createParsedLayer("layer2", LayerType.BottomCopper, {
      minX: 0,
      minY: 0,
      maxX: 10,
      maxY: 10,
    });

    manager.addLayer(layer1);
    manager.addLayer(layer2);

    let visible = manager.getVisibleLayers();
    expect(visible).toHaveLength(2);

    const layerToHide = visible[1];
    if (layerToHide !== undefined) {
      layerToHide.visible = false;
    }
    visible = manager.getVisibleLayers();
    expect(visible).toHaveLength(1);
    expect(visible[0]?.id).toBe("layer2");
  });
});

describe("SceneManager clear", () => {
  it("deletes all buffers and empties scene", () => {
    const gl = createMockGL();
    const manager = new SceneManager(gl);

    manager.addLayer(
      createParsedLayer("layer1", LayerType.TopCopper, {
        minX: 0,
        minY: 0,
        maxX: 10,
        maxY: 10,
      }),
    );
    manager.addLayer(
      createParsedLayer("layer2", LayerType.Drill, {
        minX: 0,
        minY: 0,
        maxX: 10,
        maxY: 10,
      }),
    );

    const deleteBufferCallsBefore = gl.deleteBuffer.mock.calls.length;
    manager.clear();
    const deleteBufferCallsAfter = gl.deleteBuffer.mock.calls.length;

    expect(deleteBufferCallsAfter - deleteBufferCallsBefore).toBe(4);
    expect(manager.getVisibleLayers()).toHaveLength(0);
    expect(manager.getBounds()).toBeNull();
  });
});

describe("SceneManager getBounds", () => {
  it("returns union of all layer bounds", () => {
    const gl = createMockGL();
    const manager = new SceneManager(gl);

    manager.addLayer(
      createParsedLayer("layer1", LayerType.TopCopper, {
        minX: 0,
        minY: 0,
        maxX: 10,
        maxY: 10,
      }),
    );
    manager.addLayer(
      createParsedLayer("layer2", LayerType.Drill, {
        minX: 5,
        minY: 5,
        maxX: 15,
        maxY: 15,
      }),
    );

    const bounds = manager.getBounds();
    expect(bounds).not.toBeNull();
    expect(bounds).toStrictEqual({
      minX: 0,
      minY: 0,
      maxX: 15,
      maxY: 15,
    });
  });

  it("returns null when empty", () => {
    const gl = createMockGL();
    const manager = new SceneManager(gl);

    expect(manager.getBounds()).toBeNull();
  });
});

describe("SceneManager releaseGPUResources", () => {
  it("deletes VBOs", () => {
    const gl = createMockGL();
    const manager = new SceneManager(gl);

    manager.addLayer(
      createParsedLayer("layer1", LayerType.TopCopper, {
        minX: 0,
        minY: 0,
        maxX: 10,
        maxY: 10,
      }),
    );

    const visibleBefore = manager.getVisibleLayers();
    expect(visibleBefore[0]?.renderState).not.toBeNull();

    manager.releaseGPUResources();

    expect(gl.deleteBuffer).toHaveBeenCalled();
    const visibleAfter = manager.getVisibleLayers();
    expect(visibleAfter[0]?.renderState).toBeNull();
  });
});

describe("SceneManager restoreGPUResources", () => {
  it("re-uploads VBOs", () => {
    const gl = createMockGL();
    const manager = new SceneManager(gl);

    manager.addLayer(
      createParsedLayer("layer1", LayerType.TopCopper, {
        minX: 0,
        minY: 0,
        maxX: 10,
        maxY: 10,
      }),
    );

    manager.releaseGPUResources();
    const visibleAfterRelease = manager.getVisibleLayers();
    expect(visibleAfterRelease[0]?.renderState).toBeNull();

    manager.restoreGPUResources();
    const visibleAfterRestore = manager.getVisibleLayers();
    expect(visibleAfterRestore[0]?.renderState).not.toBeNull();
    expect(visibleAfterRestore[0]?.renderState?.indexCount).toBe(3);
    expect(gl.createBuffer).toHaveBeenCalled();
    expect(gl.bufferData).toHaveBeenCalled();
  });
});
