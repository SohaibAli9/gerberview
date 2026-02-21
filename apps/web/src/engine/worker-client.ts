import type {
  CancelMessage,
  IdentifiedFile,
  LayerErrorMessage,
  LayerResultMessage,
  ParseRequestMessage,
  WorkerToMainMessage,
} from "../types";

function createRequestId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }

  return `req-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

function toOwnedArrayBuffer(content: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(content);
  return copy.buffer;
}

function shouldStopReading(complete: boolean, queueLength: number): boolean {
  return complete && queueLength === 0;
}

function toWorkerPayload(files: readonly IdentifiedFile[]): {
  requestFiles: ParseRequestMessage["files"];
  transfer: Transferable[];
} {
  const requestFiles: ParseRequestMessage["files"][number][] = [];
  const transfer: Transferable[] = [];

  for (const file of files) {
    if (file.fileType === "unknown") {
      throw new Error(`Unsupported file type for "${file.fileName}"`);
    }

    const buffer = toOwnedArrayBuffer(file.content);
    requestFiles.push({
      fileName: file.fileName,
      layerType: file.layerType,
      fileType: file.fileType,
      content: buffer,
    });
    transfer.push(buffer);
  }

  return { requestFiles, transfer };
}

/**
 * Facade over Worker postMessage/onmessage for parsing requests.
 */
export class WorkerClient {
  private readonly worker: Worker;
  private readonly readyPromise: Promise<void>;
  private activeRequestId: string | null = null;
  private activeCancelHook: (() => void) | null = null;
  private disposed = false;

  constructor() {
    this.worker = new Worker(new URL("./parse-worker.ts", import.meta.url), { type: "module" });
    this.readyPromise = new Promise<void>((resolve, reject) => {
      const onMessage = (event: MessageEvent<WorkerToMainMessage>): void => {
        if (event.data.type !== "worker-ready") {
          return;
        }

        this.worker.removeEventListener("message", onMessage);
        this.worker.removeEventListener("error", onError);
        resolve();
      };

      const onError = (event: ErrorEvent): void => {
        this.worker.removeEventListener("message", onMessage);
        this.worker.removeEventListener("error", onError);
        reject(event.error instanceof Error ? event.error : new Error(event.message));
      };

      this.worker.addEventListener("message", onMessage);
      this.worker.addEventListener("error", onError);
    });
  }

  /**
   * Wait for worker startup and WASM initialization.
   */
  async waitForReady(): Promise<void> {
    this.ensureNotDisposed();
    await this.readyPromise;
  }

  /**
   * Parse files and stream per-file results.
   */
  async *parseFiles(
    files: readonly IdentifiedFile[],
  ): AsyncGenerator<LayerResultMessage | LayerErrorMessage> {
    this.ensureNotDisposed();
    await this.waitForReady();
    this.cancel();

    const requestId = createRequestId();
    const { requestFiles, transfer } = toWorkerPayload(files);
    const message: ParseRequestMessage = {
      type: "parse-request",
      requestId,
      files: requestFiles,
    };

    this.activeRequestId = requestId;
    const queue: (LayerResultMessage | LayerErrorMessage)[] = [];
    let complete = false;
    let resolveWaiter: (() => void) | null = null;

    const wake = (): void => {
      if (resolveWaiter) {
        const resolve = resolveWaiter;
        resolveWaiter = null;
        resolve();
      }
    };

    const markCancelled = (): void => {
      complete = true;
      wake();
    };
    this.activeCancelHook = markCancelled;

    const onMessage = (event: MessageEvent<WorkerToMainMessage>): void => {
      const incoming = event.data;
      if (incoming.type === "worker-ready") {
        return;
      }

      if (incoming.requestId !== requestId || this.activeRequestId !== requestId) {
        return;
      }

      if (incoming.type === "parse-complete") {
        complete = true;
        this.activeRequestId = null;
        wake();
        return;
      }

      queue.push(incoming);
      wake();
    };

    const onError = (event: ErrorEvent): void => {
      if (this.activeRequestId !== requestId) {
        return;
      }

      queue.push({
        type: "layer-error",
        requestId,
        fileName: "<worker>",
        error: event.message || "Worker error",
      });
      complete = true;
      this.activeRequestId = null;
      wake();
    };

    this.worker.addEventListener("message", onMessage);
    this.worker.addEventListener("error", onError);
    this.worker.postMessage(message, transfer);

    try {
      for (;;) {
        if (shouldStopReading(complete, queue.length)) {
          break;
        }

        if (queue.length === 0) {
          await new Promise<void>((resolve) => {
            resolveWaiter = resolve;
          });
        }

        while (queue.length > 0) {
          const item = queue.shift();
          if (item === undefined) {
            continue;
          }
          yield item;
        }
      }
    } finally {
      this.worker.removeEventListener("message", onMessage);
      this.worker.removeEventListener("error", onError);
      if (this.activeRequestId === requestId) {
        this.worker.postMessage({ type: "cancel", requestId } satisfies CancelMessage);
        this.activeRequestId = null;
      }
      if (this.activeCancelHook === markCancelled) {
        this.activeCancelHook = null;
      }
    }
  }

  /**
   * Cancel any in-flight parse request.
   */
  cancel(): void {
    if (this.activeRequestId === null) {
      return;
    }

    const requestId = this.activeRequestId;
    this.activeRequestId = null;
    this.worker.postMessage({ type: "cancel", requestId } satisfies CancelMessage);
    if (this.activeCancelHook) {
      this.activeCancelHook();
      this.activeCancelHook = null;
    }
  }

  /**
   * Terminate worker and release client resources.
   */
  dispose(): void {
    if (this.disposed) {
      return;
    }

    this.cancel();
    this.worker.terminate();
    this.disposed = true;
  }

  private ensureNotDisposed(): void {
    if (!this.disposed) {
      return;
    }

    throw new Error("WorkerClient has been disposed");
  }
}
