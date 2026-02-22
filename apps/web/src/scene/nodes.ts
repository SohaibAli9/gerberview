import type {
  BoundingBox,
  LayerColor,
  LayerMeta,
  LayerRenderState,
  LayerType,
  ViewMatrix,
} from "../types";

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

/** The root of all board-space layers. */
export interface BoardNode extends SceneNode {
  readonly kind: "board";
  readonly layers: readonly LayerNode[];
  readonly bounds: BoundingBox;
}

/** Container for non-board overlays (cursor crosshair, future measurements). */
export interface OverlayGroup extends SceneNode {
  readonly kind: "overlay-group";
  readonly children: readonly OverlayNode[];
}

/** A single overlay element (crosshair, ruler, grid). */
export interface OverlayNode extends SceneNode {
  readonly kind: "crosshair" | "measurement" | "grid";
  readonly renderFn: (gl: WebGLRenderingContext, viewMatrix: ViewMatrix) => void;
}

/** The complete scene. */
export interface SceneRoot {
  readonly board: BoardNode | null;
  readonly overlays: OverlayGroup;
}
