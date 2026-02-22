import vertexSource from "./shaders/vertex.glsl?raw";
import fragmentSource from "./shaders/fragment.glsl?raw";

export interface ShaderUniforms {
  readonly viewMatrix: WebGLUniformLocation;
  readonly color: WebGLUniformLocation;
}

export interface ShaderAttribs {
  readonly position: number;
}

export interface ShaderProgramResult {
  readonly program: WebGLProgram;
  readonly uniforms: ShaderUniforms;
  readonly attribs: ShaderAttribs;
}

export function compileShader(
  gl: WebGLRenderingContext,
  type: GLenum,
  source: string,
): WebGLShader {
  const shader = gl.createShader(type);
  if (shader === null) {
    throw new Error("Failed to create WebGL shader");
  }

  gl.shaderSource(shader, source);
  gl.compileShader(shader);

  if (gl.getShaderParameter(shader, gl.COMPILE_STATUS) !== true) {
    const log = gl.getShaderInfoLog(shader) ?? "Unknown shader compilation error";
    gl.deleteShader(shader);
    throw new Error(`Shader compilation failed: ${log}`);
  }

  return shader;
}

export function createProgram(
  gl: WebGLRenderingContext,
  vertexShader: WebGLShader,
  fragmentShader: WebGLShader,
): WebGLProgram {
  const program = gl.createProgram();
  // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- WebGL createProgram returns null on failure
  if (program === null) {
    throw new Error("Failed to create WebGL program");
  }

  gl.attachShader(program, vertexShader);
  gl.attachShader(program, fragmentShader);
  gl.linkProgram(program);

  if (gl.getProgramParameter(program, gl.LINK_STATUS) !== true) {
    const log = gl.getProgramInfoLog(program) ?? "Unknown program link error";
    gl.deleteProgram(program);
    throw new Error(`Program link failed: ${log}`);
  }

  return program;
}

export function getUniformLocations(
  gl: WebGLRenderingContext,
  program: WebGLProgram,
): ShaderUniforms {
  const viewMatrix = gl.getUniformLocation(program, "u_viewMatrix");
  const color = gl.getUniformLocation(program, "u_color");

  if (viewMatrix === null) {
    throw new Error("Uniform u_viewMatrix not found");
  }
  if (color === null) {
    throw new Error("Uniform u_color not found");
  }

  return { viewMatrix, color };
}

export function getAttribLocations(
  gl: WebGLRenderingContext,
  program: WebGLProgram,
): ShaderAttribs {
  const position = gl.getAttribLocation(program, "a_position");

  if (position === -1) {
    throw new Error("Attribute a_position not found");
  }

  return { position };
}

export function initShaderProgram(gl: WebGLRenderingContext): ShaderProgramResult {
  const vertexShader = compileShader(gl, gl.VERTEX_SHADER, vertexSource);
  const fragmentShader = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource);

  try {
    const program = createProgram(gl, vertexShader, fragmentShader);
    const uniforms = getUniformLocations(gl, program);
    const attribs = getAttribLocations(gl, program);
    return { program, uniforms, attribs };
  } finally {
    gl.deleteShader(vertexShader);
    gl.deleteShader(fragmentShader);
  }
}
