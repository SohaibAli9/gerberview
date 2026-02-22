import type { LayerColor, LayerMeta, LayerRenderState, LayerType } from "../types";

/** Base for all scene nodes. */
export interface SceneNode {
  readonly id: string;
  visible: boolean;
}

/** A renderable layer with GPU buffer handles. */
export interface LayerNode extends SceneNode {
  readonly kind: "layer";
  readonly layerType: LayerType;
  readonly color: LayerColor;
  readonly renderState: LayerRenderState | null;
  readonly meta: LayerMeta;
  readonly zOrder: number;
  opacity: number;
}
