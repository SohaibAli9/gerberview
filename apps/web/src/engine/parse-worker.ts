import initWasm, {
  get_indices,
  get_positions,
  parse_excellon,
  parse_gerber,
} from "gerberview-wasm";
import type {
  CancelMessage,
  FilePayload,
  LayerMeta,
  MainToWorkerMessage,
  ParseRequestMessage,
  WorkerReadyMessage,
  WorkerToMainMessage,
} from "../types";

interface WorkerScopeLike {
  addEventListener(
    type: "message",
    listener: (event: MessageEvent<MainToWorkerMessage>) => void,
  ): void;
  postMessage(message: WorkerToMainMessage, transfer?: readonly Transferable[]): void;
}

interface WasmBounds {
  readonly min_x: number;
  readonly min_y: number;
  readonly max_x: number;
  readonly max_y: number;
}

interface WasmLayerMeta {
  readonly bounds: WasmBounds;
  readonly vertex_count: number;
  readonly index_count: number;
  readonly command_count: number;
  readonly warning_count: number;
  readonly warnings: readonly string[];
}

const workerScope = globalThis as unknown as WorkerScopeLike;
const cancelledRequests = new Set<string>();
let activeRequestId: string | null = null;

const wasmReadyPromise = initializeWasm();

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readObject(
  source: Record<string, unknown>,
  key: string,
  fileName: string,
): Record<string, unknown> {
  const value = source[key];
  if (isObject(value)) {
    return value;
  }

  throw new Error(`Invalid ${key} in parse result for "${fileName}"`);
}

function readNumber(source: Record<string, unknown>, key: string, fileName: string): number {
  const value = source[key];
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }

  throw new Error(`Invalid ${key} in parse result for "${fileName}"`);
}

function readWarnings(
  source: Record<string, unknown>,
  key: string,
  fileName: string,
): readonly string[] {
  const value = source[key];
  if (!Array.isArray(value)) {
    throw new Error(`Invalid ${key} in parse result for "${fileName}"`);
  }

  const warnings = value.filter((item): item is string => typeof item === "string");
  if (warnings.length !== value.length) {
    throw new Error(`Invalid ${key} in parse result for "${fileName}"`);
  }

  return warnings;
}

function toLayerMeta(value: unknown, fileName: string): LayerMeta {
  if (!isObject(value)) {
    throw new Error(`Invalid parse result for "${fileName}"`);
  }

  const rawMeta = value as unknown as WasmLayerMeta;
  const metaRecord = rawMeta as unknown as Record<string, unknown>;
  const boundsRecord = readObject(metaRecord, "bounds", fileName);

  return {
    bounds: {
      minX: readNumber(boundsRecord, "min_x", fileName),
      minY: readNumber(boundsRecord, "min_y", fileName),
      maxX: readNumber(boundsRecord, "max_x", fileName),
      maxY: readNumber(boundsRecord, "max_y", fileName),
    },
    vertexCount: readNumber(metaRecord, "vertex_count", fileName),
    indexCount: readNumber(metaRecord, "index_count", fileName),
    commandCount: readNumber(metaRecord, "command_count", fileName),
    warningCount: readNumber(metaRecord, "warning_count", fileName),
    warnings: readWarnings(metaRecord, "warnings", fileName),
  };
}

function isRequestActive(requestId: string): boolean {
  return activeRequestId === requestId && !cancelledRequests.has(requestId);
}

function toErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

async function yieldToEventLoop(): Promise<void> {
  await new Promise<void>((resolve) => {
    setTimeout(resolve, 0);
  });
}

async function initializeWasm(): Promise<void> {
  await initWasm();
  const ready: WorkerReadyMessage = { type: "worker-ready" };
  workerScope.postMessage(ready);
}

function parseFile(file: FilePayload): {
  meta: LayerMeta;
  positions: Float32Array;
  indices: Uint32Array;
} {
  const bytes = new Uint8Array(file.content);
  const rawMeta: unknown =
    file.fileType === "gerber"
      ? (parse_gerber(bytes) as unknown)
      : (parse_excellon(bytes) as unknown);
  const meta = toLayerMeta(rawMeta, file.fileName);

  return {
    meta,
    positions: get_positions(),
    indices: get_indices(),
  };
}

function postLayerResult(
  requestId: string,
  file: FilePayload,
  parsed: { meta: LayerMeta; positions: Float32Array; indices: Uint32Array },
): void {
  const transferables: Transferable[] = [];
  if (parsed.positions.buffer instanceof ArrayBuffer) {
    transferables.push(parsed.positions.buffer);
  }
  if (parsed.indices.buffer instanceof ArrayBuffer) {
    transferables.push(parsed.indices.buffer);
  }

  workerScope.postMessage(
    {
      type: "layer-result",
      requestId,
      fileName: file.fileName,
      layerType: file.layerType,
      meta: parsed.meta,
      positions: parsed.positions,
      indices: parsed.indices,
    },
    transferables,
  );
}

async function processParseRequest(message: ParseRequestMessage): Promise<void> {
  let successCount = 0;
  let errorCount = 0;
  let totalWarnings = 0;
  const startedAt = performance.now();

  try {
    await wasmReadyPromise;
  } catch (error: unknown) {
    if (!isRequestActive(message.requestId)) {
      return;
    }

    errorCount = message.files.length;
    const errorMessage = `WASM initialization failed: ${toErrorMessage(error)}`;
    for (const file of message.files) {
      workerScope.postMessage({
        type: "layer-error",
        requestId: message.requestId,
        fileName: file.fileName,
        error: errorMessage,
      });
    }
  }

  if (errorCount === 0) {
    for (const file of message.files) {
      if (!isRequestActive(message.requestId)) {
        break;
      }

      try {
        const parsed = parseFile(file);
        if (!isRequestActive(message.requestId)) {
          break;
        }

        postLayerResult(message.requestId, file, parsed);
        successCount += 1;
        totalWarnings += parsed.meta.warningCount;
      } catch (error: unknown) {
        if (!isRequestActive(message.requestId)) {
          break;
        }

        workerScope.postMessage({
          type: "layer-error",
          requestId: message.requestId,
          fileName: file.fileName,
          error: toErrorMessage(error),
        });
        errorCount += 1;
      }

      await yieldToEventLoop();
    }
  }

  if (isRequestActive(message.requestId)) {
    workerScope.postMessage({
      type: "parse-complete",
      requestId: message.requestId,
      totalLayers: message.files.length,
      successCount,
      errorCount,
      totalWarnings,
      elapsedMs: performance.now() - startedAt,
    });
  }

  if (activeRequestId === message.requestId) {
    activeRequestId = null;
  }
  cancelledRequests.delete(message.requestId);
}

function handleCancel(message: CancelMessage): void {
  cancelledRequests.add(message.requestId);
  if (activeRequestId === message.requestId) {
    activeRequestId = null;
  }
}

function handleMessage(event: MessageEvent<MainToWorkerMessage>): void {
  const message = event.data;

  if (message.type === "cancel") {
    handleCancel(message);
    return;
  }

  activeRequestId = message.requestId;
  cancelledRequests.delete(message.requestId);
  void processParseRequest(message);
}

workerScope.addEventListener("message", handleMessage);
