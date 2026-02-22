import { DEFAULT_VIEWER_CONFIG } from "../constants";
import type { SceneManager } from "../scene/scene";
import type { BoundingBox, ViewMatrix, ViewState } from "../types";
import { initShaderProgram } from "./shader";

export interface RendererOptions {
  readonly backgroundColor?: readonly [number, number, number, number];
}

const IDENTITY_MATRIX: ViewMatrix = [1, 0, 0, 0, 1, 0, 0, 0, 1];

function computeViewMatrix(
  viewState: ViewState,
  boardBounds: BoundingBox | null,
  canvasWidth: number,
  canvasHeight: number,
): ViewMatrix {
  if (boardBounds === null || canvasWidth <= 0 || canvasHeight <= 0) {
    return IDENTITY_MATRIX;
  }

  const boardWidth = boardBounds.maxX - boardBounds.minX;
  const boardHeight = boardBounds.maxY - boardBounds.minY;

  if (boardWidth <= 0 || boardHeight <= 0) {
    return IDENTITY_MATRIX;
  }

  const padding = DEFAULT_VIEWER_CONFIG.fitPadding;
  const scaleX = canvasWidth / (boardWidth * (1 + 2 * padding));
  const scaleY = canvasHeight / (boardHeight * (1 + 2 * padding));
  const baseScale = Math.min(scaleX, scaleY);
  const finalScale = baseScale * viewState.zoom;

  const offsetX = (-viewState.centerX * finalScale * 2) / canvasWidth;
  const offsetY = (-viewState.centerY * finalScale * 2) / canvasHeight;

  const scaleX2 = (finalScale * 2) / canvasWidth;
  const scaleY2 = (finalScale * 2) / canvasHeight;

  return [scaleX2, 0, 0, 0, scaleY2, 0, offsetX, offsetY, 1];
}

/** On-demand WebGL renderer using dirty-flag pattern. */
export class Renderer {
  private readonly gl: WebGLRenderingContext;
  private readonly sceneManager: SceneManager;
  private readonly canvas: HTMLCanvasElement;
  private readonly backgroundColor: readonly [number, number, number, number];

  private program: WebGLProgram;
  private uniforms: { viewMatrix: WebGLUniformLocation; color: WebGLUniformLocation };
  private attribs: { position: number };

  private viewState: ViewState = {
    centerX: 0,
    centerY: 0,
    zoom: 1,
  };
  private boardBounds: BoundingBox | null = null;
  private globalOpacity = 1.0;

  private dirty = false;
  private rafId: number | null = null;

  constructor(
    gl: WebGLRenderingContext,
    sceneManager: SceneManager,
    canvas: HTMLCanvasElement,
    options?: RendererOptions,
  ) {
    this.gl = gl;
    this.sceneManager = sceneManager;
    this.canvas = canvas;
    this.backgroundColor = options?.backgroundColor ?? DEFAULT_VIEWER_CONFIG.backgroundColor;

    const ext = gl.getExtension("OES_element_index_uint");
    if (ext === null) {
      throw new Error("Browser does not support required WebGL extensions");
    }

    const { program, uniforms, attribs } = initShaderProgram(gl);
    this.program = program;
    this.uniforms = uniforms;
    this.attribs = attribs;
  }

  /** Mark the scene as needing a redraw. */
  markDirty(): void {
    if (!this.dirty) {
      this.dirty = true;
      this.scheduleFrame();
    }
  }

  private scheduleFrame(): void {
    this.rafId ??= requestAnimationFrame(() => {
      this.renderFrame();
    });
  }

  private renderFrame(): void {
    this.rafId = null;
    if (!this.dirty) {
      return;
    }
    this.dirty = false;
    this.draw();
  }

  /** Run the draw sequence. Skips layer drawing if no visible layers. */
  draw(): void {
    const { gl, canvas } = this;
    const layers = this.sceneManager.getVisibleLayers();

    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.clearColor(
      this.backgroundColor[0],
      this.backgroundColor[1],
      this.backgroundColor[2],
      this.backgroundColor[3],
    );
    gl.clear(gl.COLOR_BUFFER_BIT);

    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.enable(gl.BLEND);

    const viewMatrix = computeViewMatrix(
      this.viewState,
      this.boardBounds,
      canvas.width,
      canvas.height,
    );

    gl.useProgram(this.program);
    gl.uniformMatrix3fv(this.uniforms.viewMatrix, false, viewMatrix);

    for (const node of layers) {
      if (node.renderState === null) {
        continue;
      }

      gl.bindBuffer(gl.ARRAY_BUFFER, node.renderState.positionVBO);
      gl.vertexAttribPointer(this.attribs.position, 2, gl.FLOAT, false, 0, 0);
      gl.enableVertexAttribArray(this.attribs.position);

      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, node.renderState.indexVBO);

      const finalA = node.color.a * node.opacity * this.globalOpacity;
      const finalColor: [number, number, number, number] = [
        node.color.r,
        node.color.g,
        node.color.b,
        finalA,
      ];
      gl.uniform4fv(this.uniforms.color, finalColor);

      gl.drawElements(gl.TRIANGLES, node.renderState.indexCount, gl.UNSIGNED_INT, 0);
    }
  }

  /** Recreate shader program. Call after WebGL context restore. */
  recompileShaders(): void {
    const { program, uniforms, attribs } = initShaderProgram(this.gl);
    this.program = program;
    this.uniforms = uniforms;
    this.attribs = attribs;
  }

  /** Set view state for view matrix computation. */
  setViewState(state: ViewState): void {
    this.viewState = { ...state };
  }

  /** Set board bounds for view matrix computation. */
  setBoardBounds(bounds: BoundingBox | null): void {
    this.boardBounds = bounds;
  }

  /** Set global opacity multiplier [0, 1]. */
  setGlobalOpacity(opacity: number): void {
    this.globalOpacity = opacity;
  }

  /** Update viewport. Call when canvas resizes. */
  resize(width: number, height: number): void {
    this.gl.viewport(0, 0, width, height);
  }
}
