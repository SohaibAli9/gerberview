import { LAYER_COLORS, LAYER_Z_ORDER } from "../constants";
import type { BoundingBox, LayerColor, LayerRenderState, ParsedLayer } from "../types";
import type { LayerNode } from "./nodes";

interface StoredBufferData {
  readonly positionBuffer: Float32Array;
  readonly indexBuffer: Uint32Array;
}

function unionBounds(a: BoundingBox, b: BoundingBox): BoundingBox {
  return {
    minX: Math.min(a.minX, b.minX),
    minY: Math.min(a.minY, b.minY),
    maxX: Math.max(a.maxX, b.maxX),
    maxY: Math.max(a.maxY, b.maxY),
  };
}

/** Manages the scene graph lifecycle: add layers, remove, sync with store. */
export class SceneManager {
  private readonly gl: WebGLRenderingContext;
  private layers: LayerNode[] = [];
  private readonly bufferData = new Map<string, StoredBufferData>();

  constructor(gl: WebGLRenderingContext) {
    this.gl = gl;
  }

  /** Add a parsed layer: upload VBOs, create LayerNode, insert in z-order. */
  addLayer(layer: ParsedLayer): void {
    const positionVBO = this.gl.createBuffer();
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- WebGL createBuffer returns null on context loss
    if (positionVBO === null) {
      throw new Error("Failed to create position VBO");
    }
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, positionVBO);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, layer.positionBuffer, this.gl.STATIC_DRAW);

    const indexVBO = this.gl.createBuffer();
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- WebGL createBuffer returns null on context loss
    if (indexVBO === null) {
      this.gl.deleteBuffer(positionVBO);
      throw new Error("Failed to create index VBO");
    }
    this.gl.bindBuffer(this.gl.ELEMENT_ARRAY_BUFFER, indexVBO);
    this.gl.bufferData(this.gl.ELEMENT_ARRAY_BUFFER, layer.indexBuffer, this.gl.STATIC_DRAW);

    const renderState: LayerRenderState = {
      positionVBO,
      indexVBO,
      indexCount: layer.indexBuffer.length,
    };

    this.bufferData.set(layer.id, {
      positionBuffer: layer.positionBuffer,
      indexBuffer: layer.indexBuffer,
    });

    const zOrder = LAYER_Z_ORDER[layer.layerType];
    const color: LayerColor = LAYER_COLORS[layer.layerType];

    const node: LayerNode = {
      kind: "layer",
      id: layer.id,
      visible: layer.visible,
      layerType: layer.layerType,
      color,
      renderState,
      meta: layer.meta,
      zOrder,
      opacity: layer.opacity,
    };

    const insertIndex = this.layers.findIndex((l) => l.zOrder > zOrder);
    if (insertIndex === -1) {
      this.layers.push(node);
    } else {
      this.layers.splice(insertIndex, 0, node);
    }
  }

  /** Remove all layers and release GPU resources. */
  clear(): void {
    for (const layer of this.layers) {
      if (layer.renderState !== null) {
        this.gl.deleteBuffer(layer.renderState.positionVBO);
        this.gl.deleteBuffer(layer.renderState.indexVBO);
      }
    }
    this.layers = [];
    this.bufferData.clear();
  }

  /** Return all visible LayerNodes sorted by z-order (back to front). */
  getVisibleLayers(): readonly LayerNode[] {
    return this.layers.filter((l) => l.visible).sort((a, b) => a.zOrder - b.zOrder);
  }

  /** Return the union bounding box of all layers. */
  getBounds(): BoundingBox | null {
    if (this.layers.length === 0) {
      return null;
    }
    const first = this.layers[0];
    if (first === undefined) {
      return null;
    }
    let bounds: BoundingBox = first.meta.bounds;
    for (let i = 1; i < this.layers.length; i++) {
      const layer = this.layers[i];
      if (layer !== undefined) {
        bounds = unionBounds(bounds, layer.meta.bounds);
      }
    }
    return bounds;
  }

  /** Release all WebGL buffers. Called on context loss. */
  releaseGPUResources(): void {
    this.layers.forEach((layer, i) => {
      if (layer.renderState !== null) {
        this.gl.deleteBuffer(layer.renderState.positionVBO);
        this.gl.deleteBuffer(layer.renderState.indexVBO);
        const updated: LayerNode = {
          ...layer,
          renderState: null,
        };
        this.layers[i] = updated;
      }
    });
  }

  /** Re-upload all VBOs. Called on context restore. */
  restoreGPUResources(): void {
    this.layers.forEach((layer, i) => {
      const stored = this.bufferData.get(layer.id);
      if (stored === undefined) {
        return;
      }

      const positionVBO = this.gl.createBuffer();
      // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- WebGL createBuffer returns null on context loss
      if (positionVBO === null) {
        throw new Error("Failed to create position VBO during restore");
      }
      this.gl.bindBuffer(this.gl.ARRAY_BUFFER, positionVBO);
      this.gl.bufferData(this.gl.ARRAY_BUFFER, stored.positionBuffer, this.gl.STATIC_DRAW);

      const indexVBO = this.gl.createBuffer();
      // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- WebGL createBuffer returns null on context loss
      if (indexVBO === null) {
        this.gl.deleteBuffer(positionVBO);
        throw new Error("Failed to create index VBO during restore");
      }
      this.gl.bindBuffer(this.gl.ELEMENT_ARRAY_BUFFER, indexVBO);
      this.gl.bufferData(this.gl.ELEMENT_ARRAY_BUFFER, stored.indexBuffer, this.gl.STATIC_DRAW);

      const renderState: LayerRenderState = {
        positionVBO,
        indexVBO,
        indexCount: stored.indexBuffer.length,
      };

      const updated: LayerNode = {
        ...layer,
        renderState,
      };
      this.layers[i] = updated;
    });
  }
}
