// ── Const enum objects ──────────────────────────────────────────────

export const LayerType = {
  TopCopper: "top_copper",
  BottomCopper: "bottom_copper",
  TopSolderMask: "top_solder_mask",
  BottomSolderMask: "bottom_solder_mask",
  TopSilkscreen: "top_silkscreen",
  BottomSilkscreen: "bottom_silkscreen",
  TopPaste: "top_paste",
  BottomPaste: "bottom_paste",
  BoardOutline: "board_outline",
  Drill: "drill",
  InnerCopper: "inner_copper",
  Unknown: "unknown",
} as const;

export type LayerType = (typeof LayerType)[keyof typeof LayerType];

export const AppState = {
  Empty: "empty",
  Loading: "loading",
  Rendered: "rendered",
  Error: "error",
} as const;

export type AppState = (typeof AppState)[keyof typeof AppState];

export const ErrorCode = {
  InvalidFileType: "INVALID_FILE_TYPE",
  EmptyZip: "EMPTY_ZIP",
  NoGerberFiles: "NO_GERBER_FILES",
  ZipTooLarge: "ZIP_TOO_LARGE",
  ParseFailed: "PARSE_FAILED",
  WebGLUnavailable: "WEBGL_UNAVAILABLE",
  WasmLoadFailed: "WASM_LOAD_FAILED",
} as const;

export type ErrorCode = (typeof ErrorCode)[keyof typeof ErrorCode];

// ── Core geometry ───────────────────────────────────────────────────

export interface Point {
  readonly x: number;
  readonly y: number;
}

export interface BoundingBox {
  readonly minX: number;
  readonly minY: number;
  readonly maxX: number;
  readonly maxY: number;
}

export interface LayerColor {
  readonly r: number;
  readonly g: number;
  readonly b: number;
  readonly a: number;
}

// ── Layer types ─────────────────────────────────────────────────────

export interface LayerMeta {
  readonly bounds: BoundingBox;
  readonly vertexCount: number;
  readonly indexCount: number;
  readonly commandCount: number;
  readonly warningCount: number;
  readonly warnings: readonly string[];
}

export interface ParsedLayer {
  readonly id: string;
  readonly fileName: string;
  readonly layerType: LayerType;
  readonly color: LayerColor;
  readonly meta: LayerMeta;
  readonly positionBuffer: Float32Array;
  readonly indexBuffer: Uint32Array;
  visible: boolean;
  opacity: number;
}

export interface LayerRenderState {
  readonly positionVBO: WebGLBuffer;
  readonly indexVBO: WebGLBuffer;
  readonly indexCount: number;
}

export interface IdentifiedFile {
  readonly fileName: string;
  readonly layerType: LayerType;
  readonly fileType: "gerber" | "excellon" | "unknown";
  readonly content: Uint8Array;
}

// ── View types ──────────────────────────────────────────────────────

export type ViewMatrix = readonly [
  number,
  number,
  number,
  number,
  number,
  number,
  number,
  number,
  number,
];

export interface ViewState {
  centerX: number;
  centerY: number;
  zoom: number;
}

export interface ViewerConfig {
  readonly minZoom: number;
  readonly maxZoom: number;
  readonly zoomFactor: number;
  readonly fitPadding: number;
  readonly backgroundColor: readonly [number, number, number, number];
}

// ── Application state ───────────────────────────────────────────────

export interface AppError {
  readonly code: ErrorCode;
  readonly message: string;
  readonly details?: string | undefined;
}

export interface LoadingProgress {
  readonly current: number;
  readonly total: number;
  readonly label: string;
}

// ── Worker message types (Main → Worker) ────────────────────────────

export interface FilePayload {
  readonly fileName: string;
  readonly layerType: LayerType;
  readonly fileType: "gerber" | "excellon";
  readonly content: ArrayBuffer;
}

export interface ParseRequestMessage {
  readonly type: "parse-request";
  readonly requestId: string;
  readonly files: readonly FilePayload[];
}

export interface CancelMessage {
  readonly type: "cancel";
  readonly requestId: string;
}

export type MainToWorkerMessage = ParseRequestMessage | CancelMessage;

// ── Worker message types (Worker → Main) ────────────────────────────

export interface WorkerReadyMessage {
  readonly type: "worker-ready";
}

export interface LayerResultMessage {
  readonly type: "layer-result";
  readonly requestId: string;
  readonly fileName: string;
  readonly layerType: LayerType;
  readonly meta: LayerMeta;
  readonly positions: Float32Array;
  readonly indices: Uint32Array;
}

export interface LayerErrorMessage {
  readonly type: "layer-error";
  readonly requestId: string;
  readonly fileName: string;
  readonly error: string;
}

export interface ParseCompleteMessage {
  readonly type: "parse-complete";
  readonly requestId: string;
  readonly totalLayers: number;
  readonly successCount: number;
  readonly errorCount: number;
  readonly totalWarnings: number;
  readonly elapsedMs: number;
}

export type WorkerToMainMessage =
  | WorkerReadyMessage
  | LayerResultMessage
  | LayerErrorMessage
  | ParseCompleteMessage;
