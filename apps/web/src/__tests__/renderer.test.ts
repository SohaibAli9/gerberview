import { describe, expect, it, vi } from "vitest";
import { Renderer } from "../render/renderer";
import { compileShader, createProgram, initShaderProgram } from "../render/shader";
import { SceneManager } from "../scene/scene";
import type { BoundingBox, LayerColor, LayerMeta, ParsedLayer } from "../types";
import { LayerType } from "../types";

const ARRAY_BUFFER = 0x8892;
const ELEMENT_ARRAY_BUFFER = 0x8893;
const STATIC_DRAW = 0x88e4;
const FLOAT = 0x1406;
const TRIANGLES = 0x0004;
const UNSIGNED_INT = 0x1405;
const SRC_ALPHA = 0x0302;
const ONE_MINUS_SRC_ALPHA = 0x0303;
const BLEND = 0x0be2;
const COLOR_BUFFER_BIT = 0x4000;

interface MockGL extends WebGLRenderingContext {
  createShader: ReturnType<typeof vi.fn>;
  shaderSource: ReturnType<typeof vi.fn>;
  compileShader: ReturnType<typeof vi.fn>;
  getShaderParameter: ReturnType<typeof vi.fn>;
  getShaderInfoLog: ReturnType<typeof vi.fn>;
  createProgram: ReturnType<typeof vi.fn>;
  attachShader: ReturnType<typeof vi.fn>;
  linkProgram: ReturnType<typeof vi.fn>;
  getProgramParameter: ReturnType<typeof vi.fn>;
  getProgramInfoLog: ReturnType<typeof vi.fn>;
  getUniformLocation: ReturnType<typeof vi.fn>;
  getAttribLocation: ReturnType<typeof vi.fn>;
  deleteShader: ReturnType<typeof vi.fn>;
  deleteProgram: ReturnType<typeof vi.fn>;
  createBuffer: ReturnType<typeof vi.fn>;
  bindBuffer: ReturnType<typeof vi.fn>;
  bufferData: ReturnType<typeof vi.fn>;
  deleteBuffer: ReturnType<typeof vi.fn>;
  getExtension: ReturnType<typeof vi.fn>;
  viewport: ReturnType<typeof vi.fn>;
  clearColor: ReturnType<typeof vi.fn>;
  clear: ReturnType<typeof vi.fn>;
  blendFunc: ReturnType<typeof vi.fn>;
  enable: ReturnType<typeof vi.fn>;
  useProgram: ReturnType<typeof vi.fn>;
  uniformMatrix3fv: ReturnType<typeof vi.fn>;
  uniform4fv: ReturnType<typeof vi.fn>;
  vertexAttribPointer: ReturnType<typeof vi.fn>;
  enableVertexAttribArray: ReturnType<typeof vi.fn>;
  drawElements: ReturnType<typeof vi.fn>;
}

function createMockGL(): MockGL {
  const createShader = vi.fn(() => ({}));
  const shaderSource = vi.fn();
  const compileShaderFn = vi.fn();
  const getShaderParameter = vi.fn(() => true);
  const getShaderInfoLog = vi.fn(() => "");
  const createProgramFn = vi.fn(() => ({}));
  const attachShader = vi.fn();
  const linkProgram = vi.fn();
  const getProgramParameter = vi.fn(() => true);
  const getProgramInfoLog = vi.fn(() => "");
  const getUniformLocation = vi.fn(() => ({}));
  const getAttribLocation = vi.fn(() => 0);
  const deleteShader = vi.fn();
  const deleteProgram = vi.fn();

  let bufferId = 0;
  const createBuffer = vi.fn(() => {
    bufferId += 1;
    return { __bufferId: bufferId } as unknown as WebGLBuffer;
  });
  const bindBuffer = vi.fn();
  const bufferData = vi.fn();
  const deleteBuffer = vi.fn();

  const getExtension = vi.fn((name: string) => (name === "OES_element_index_uint" ? {} : null));
  const viewport = vi.fn();
  const clearColor = vi.fn();
  const clear = vi.fn();
  const blendFunc = vi.fn();
  const enable = vi.fn();
  const useProgram = vi.fn();
  const uniformMatrix3fv = vi.fn();
  const uniform4fv = vi.fn();
  const vertexAttribPointer = vi.fn();
  const enableVertexAttribArray = vi.fn();
  const drawElements = vi.fn();

  const gl = {
    createShader,
    shaderSource,
    compileShader: compileShaderFn,
    getShaderParameter,
    getShaderInfoLog,
    createProgram: createProgramFn,
    attachShader,
    linkProgram,
    getProgramParameter,
    getProgramInfoLog,
    getUniformLocation,
    getAttribLocation,
    deleteShader,
    deleteProgram,
    createBuffer,
    bindBuffer,
    bufferData,
    deleteBuffer,
    getExtension,
    viewport,
    clearColor,
    clear,
    blendFunc,
    enable,
    useProgram,
    uniformMatrix3fv,
    uniform4fv,
    vertexAttribPointer,
    enableVertexAttribArray,
    drawElements,
    VERTEX_SHADER: 0x8b31,
    FRAGMENT_SHADER: 0x8b30,
    COMPILE_STATUS: 0x8b81,
    LINK_STATUS: 0x8b82,
    ARRAY_BUFFER,
    ELEMENT_ARRAY_BUFFER,
    STATIC_DRAW,
    FLOAT,
    TRIANGLES,
    UNSIGNED_INT,
    SRC_ALPHA,
    ONE_MINUS_SRC_ALPHA,
    BLEND,
    COLOR_BUFFER_BIT,
  } as unknown as MockGL;

  return gl;
}

function createParsedLayer(
  id: string,
  layerType: (typeof LayerType)[keyof typeof LayerType],
  bounds: BoundingBox,
): ParsedLayer {
  const positionBuffer = new Float32Array([0, 0, 1, 0, 0, 1]);
  const indexBuffer = new Uint32Array([0, 1, 2]);
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
    visible: true,
    opacity: 1,
  };
}

describe("shader compilation", () => {
  it("compileShader compiles both vertex and fragment shaders", () => {
    const gl = createMockGL();
    const vertexSource = "attribute vec2 a_position; void main() {}";
    const fragmentSource = "precision mediump float; void main() {}";

    const vertexShader = compileShader(gl, gl.VERTEX_SHADER, vertexSource);
    const fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource);

    expect(vertexShader).toBeDefined();
    expect(fragmentShader).toBeDefined();
    expect(gl.createShader).toHaveBeenCalledWith(0x8b31);
    expect(gl.createShader).toHaveBeenCalledWith(0x8b30);
    expect(gl.shaderSource).toHaveBeenCalledWith(vertexShader, vertexSource);
    expect(gl.shaderSource).toHaveBeenCalledWith(fragmentShader, fragmentSource);
    expect(gl.compileShader).toHaveBeenCalledWith(vertexShader);
    expect(gl.compileShader).toHaveBeenCalledWith(fragmentShader);
  });

  it("createProgram links shaders into a program", () => {
    const gl = createMockGL();
    const vertexShader = {} as WebGLShader;
    const fragmentShader = {} as WebGLShader;
    const programObj = {} as WebGLProgram;
    vi.mocked(gl.createProgram).mockReturnValue(programObj);

    const program = createProgram(gl, vertexShader, fragmentShader);

    expect(program).toBe(programObj);
    expect(gl.createProgram).toHaveBeenCalledOnce();
    expect(gl.attachShader).toHaveBeenCalledWith(program, vertexShader);
    expect(gl.attachShader).toHaveBeenCalledWith(program, fragmentShader);
    expect(gl.linkProgram).toHaveBeenCalledWith(program);
  });

  it("compileShader throws on compile failure with GLSL error log", () => {
    const gl = createMockGL();
    vi.mocked(gl.getShaderParameter).mockReturnValue(false);
    vi.mocked(gl.getShaderInfoLog).mockReturnValue("GLSL error message");

    expect(() => compileShader(gl, gl.VERTEX_SHADER, "invalid")).toThrow("GLSL error message");
    expect(gl.deleteShader).toHaveBeenCalled();
  });
});

describe("initShaderProgram", () => {
  it("compiles shaders, creates program, and returns all locations", () => {
    const gl = createMockGL();

    const result = initShaderProgram(gl);

    expect(result.program).toBeDefined();
    expect(result.uniforms.viewMatrix).toBeDefined();
    expect(result.uniforms.color).toBeDefined();
    expect(result.attribs.position).toBe(0);
    expect(gl.getUniformLocation).toHaveBeenCalledWith(result.program, "u_viewMatrix");
    expect(gl.getUniformLocation).toHaveBeenCalledWith(result.program, "u_color");
    expect(gl.getAttribLocation).toHaveBeenCalledWith(result.program, "a_position");
  });
});

function createRendererTestContext(): {
  gl: MockGL;
  sceneManager: SceneManager;
  canvas: HTMLCanvasElement;
  rafMock: ReturnType<typeof vi.fn>;
} {
  const gl = createMockGL();
  const sceneManager = new SceneManager(gl as unknown as WebGLRenderingContext);
  const canvas = { width: 800, height: 600 } as HTMLCanvasElement;
  const rafMock = vi.fn(() => 1);
  vi.stubGlobal("requestAnimationFrame", rafMock);
  return { gl, sceneManager, canvas, rafMock };
}

describe("Renderer markDirty", () => {
  it("markDirty twice schedules one rAF", () => {
    const { gl, sceneManager, canvas, rafMock } = createRendererTestContext();
    const renderer = new Renderer(gl as unknown as WebGLRenderingContext, sceneManager, canvas);
    renderer.draw();
    renderer.markDirty();
    renderer.markDirty();

    expect(rafMock).toHaveBeenCalledTimes(1);
  });

  it("markDirty when already dirty does not schedule another rAF", () => {
    const { gl, sceneManager, canvas, rafMock } = createRendererTestContext();
    const renderer = new Renderer(gl as unknown as WebGLRenderingContext, sceneManager, canvas);
    renderer.markDirty();
    const callsAfterFirstMarkDirty = rafMock.mock.calls.length;
    renderer.markDirty();
    renderer.markDirty();

    expect(rafMock.mock.calls.length).toBe(callsAfterFirstMarkDirty);
  });
});

describe("Renderer draw", () => {
  it("draw with 0 layers does not throw", () => {
    const { gl, sceneManager, canvas } = createRendererTestContext();
    const renderer = new Renderer(gl as unknown as WebGLRenderingContext, sceneManager, canvas);

    expect(() => {
      renderer.draw();
    }).not.toThrow();
  });

  it("draw with mock layers invokes correct GL calls", () => {
    const { gl, sceneManager, canvas } = createRendererTestContext();
    const layer = createParsedLayer("layer1", LayerType.TopCopper, {
      minX: 0,
      minY: 0,
      maxX: 10,
      maxY: 10,
    });
    sceneManager.addLayer(layer);

    const renderer = new Renderer(gl as unknown as WebGLRenderingContext, sceneManager, canvas);
    renderer.setViewState({ centerX: 5, centerY: 5, zoom: 1 });
    renderer.setBoardBounds({ minX: 0, minY: 0, maxX: 10, maxY: 10 });
    renderer.draw();

    expect(gl.viewport).toHaveBeenCalledWith(0, 0, 800, 600);
    expect(gl.clearColor).toHaveBeenCalled();
    expect(gl.clear).toHaveBeenCalledWith(COLOR_BUFFER_BIT);
    expect(gl.blendFunc).toHaveBeenCalledWith(SRC_ALPHA, ONE_MINUS_SRC_ALPHA);
    expect(gl.useProgram).toHaveBeenCalled();
    expect(gl.uniformMatrix3fv).toHaveBeenCalled();
    expect(gl.bindBuffer).toHaveBeenCalledWith(ARRAY_BUFFER, expect.anything());
    expect(gl.bindBuffer).toHaveBeenCalledWith(ELEMENT_ARRAY_BUFFER, expect.anything());
    expect(gl.vertexAttribPointer).toHaveBeenCalledWith(0, 2, FLOAT, false, 0, 0);
    expect(gl.uniform4fv).toHaveBeenCalled();
    expect(gl.drawElements).toHaveBeenCalledWith(TRIANGLES, 3, UNSIGNED_INT, 0);
  });
});

describe("Renderer resize and recompileShaders", () => {
  it("resize updates viewport", () => {
    const { gl, sceneManager, canvas } = createRendererTestContext();
    const renderer = new Renderer(gl as unknown as WebGLRenderingContext, sceneManager, canvas);

    renderer.resize(800, 600);

    expect(gl.viewport).toHaveBeenCalledWith(0, 0, 800, 600);
  });

  it("recompileShaders reinitializes program", () => {
    const { gl, sceneManager, canvas } = createRendererTestContext();
    const renderer = new Renderer(gl as unknown as WebGLRenderingContext, sceneManager, canvas);
    const createShaderCallsBefore = gl.createShader.mock.calls.length;
    const createProgramCallsBefore = gl.createProgram.mock.calls.length;

    renderer.recompileShaders();

    expect(gl.createShader.mock.calls.length).toBeGreaterThan(createShaderCallsBefore);
    expect(gl.createProgram.mock.calls.length).toBeGreaterThan(createProgramCallsBefore);
  });
});
