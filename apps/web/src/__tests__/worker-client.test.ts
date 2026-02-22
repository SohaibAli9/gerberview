import { afterEach, beforeEach, expect, it } from "vitest";
import {
  LayerType,
  type IdentifiedFile,
  type ParseCompleteMessage,
  type ParseRequestMessage,
  type WorkerToMainMessage,
} from "../types";
import { WorkerClient } from "../engine/worker-client";

const ORIGINAL_WORKER = globalThis.Worker;

interface PostedMessage {
  readonly message: unknown;
  readonly transferCount: number;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isParseRequest(value: unknown): value is ParseRequestMessage {
  if (!isRecord(value)) {
    return false;
  }

  return value["type"] === "parse-request" && typeof value["requestId"] === "string";
}

function asMessageListener(
  listener: EventListenerOrEventListenerObject | null,
): ((event: MessageEvent<WorkerToMainMessage>) => void) | null {
  if (listener === null || typeof listener !== "function") {
    return null;
  }

  return listener as (event: MessageEvent<WorkerToMainMessage>) => void;
}

function asErrorListener(
  listener: EventListenerOrEventListenerObject | null,
): ((event: ErrorEvent) => void) | null {
  if (listener === null || typeof listener !== "function") {
    return null;
  }

  return listener as (event: ErrorEvent) => void;
}

class MockWorker {
  static instances: MockWorker[] = [];

  readonly postedMessages: PostedMessage[] = [];
  terminated = false;
  private readonly messageListeners = new Set<(event: MessageEvent<WorkerToMainMessage>) => void>();
  private readonly errorListeners = new Set<(event: ErrorEvent) => void>();

  constructor(scriptUrl: string | URL, options?: WorkerOptions) {
    void scriptUrl;
    void options;
    MockWorker.instances.push(this);
  }

  postMessage(
    message: unknown,
    transferOrOptions?: readonly Transferable[] | StructuredSerializeOptions,
  ): void {
    let transferCount = 0;
    if (Array.isArray(transferOrOptions)) {
      transferCount = transferOrOptions.length;
    } else if (isRecord(transferOrOptions) && Array.isArray(transferOrOptions["transfer"])) {
      transferCount = transferOrOptions["transfer"].length;
    }

    this.postedMessages.push({ message, transferCount });
  }

  terminate(): void {
    this.terminated = true;
  }

  addEventListener(type: string, listener: EventListenerOrEventListenerObject | null): void {
    if (type === "message") {
      const fn = asMessageListener(listener);
      if (fn !== null) {
        this.messageListeners.add(fn);
      }
      return;
    }

    if (type === "error") {
      const fn = asErrorListener(listener);
      if (fn !== null) {
        this.errorListeners.add(fn);
      }
    }
  }

  removeEventListener(type: string, listener: EventListenerOrEventListenerObject | null): void {
    if (type === "message") {
      const fn = asMessageListener(listener);
      if (fn !== null) {
        this.messageListeners.delete(fn);
      }
      return;
    }

    if (type === "error") {
      const fn = asErrorListener(listener);
      if (fn !== null) {
        this.errorListeners.delete(fn);
      }
    }
  }

  emitMessage(message: WorkerToMainMessage): void {
    const event = { data: message } as MessageEvent<WorkerToMainMessage>;
    for (const listener of this.messageListeners) {
      listener(event);
    }
  }

  latestParseRequest(): ParseRequestMessage | undefined {
    const parseRequests = this.postedMessages
      .map((entry) => entry.message)
      .filter((message): message is ParseRequestMessage => isParseRequest(message));

    return parseRequests.at(-1);
  }

  hasCancelFor(requestId: string): boolean {
    return this.postedMessages.some((entry) => {
      if (!isRecord(entry.message)) {
        return false;
      }

      return entry.message["type"] === "cancel" && entry.message["requestId"] === requestId;
    });
  }
}

function latestWorker(): MockWorker {
  const worker = MockWorker.instances.at(-1);
  if (worker === undefined) {
    throw new Error("Expected MockWorker instance to exist");
  }
  return worker;
}

function createFile(fileName = "board.GTL"): IdentifiedFile {
  return {
    fileName,
    layerType: LayerType.TopCopper,
    fileType: "gerber",
    content: new Uint8Array([0x25, 0x46, 0x53, 0x4c, 0x41, 0x58]),
  };
}

function layerResult(requestId: string): WorkerToMainMessage {
  return {
    type: "layer-result",
    requestId,
    fileName: "board.GTL",
    layerType: LayerType.TopCopper,
    meta: {
      bounds: { minX: 0, minY: 0, maxX: 10, maxY: 20 },
      vertexCount: 4,
      indexCount: 6,
      commandCount: 1,
      warningCount: 0,
      warnings: [],
    },
    positions: new Float32Array([0, 0, 10, 0, 10, 20, 0, 20]),
    indices: new Uint32Array([0, 1, 2, 0, 2, 3]),
    clearRanges: new Uint32Array([]),
  };
}

function parseComplete(requestId: string): ParseCompleteMessage {
  return {
    type: "parse-complete",
    requestId,
    totalLayers: 1,
    successCount: 1,
    errorCount: 0,
    totalWarnings: 0,
    elapsedMs: 5,
  };
}

function expectYielded<T>(result: IteratorResult<T, unknown>): T {
  if (result.done === true) {
    throw new Error("Expected yielded value");
  }

  return result.value;
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

beforeEach(() => {
  MockWorker.instances = [];
  (globalThis as { Worker: typeof Worker }).Worker = MockWorker as unknown as typeof Worker;
});

afterEach(() => {
  (globalThis as { Worker: typeof Worker }).Worker = ORIGINAL_WORKER;
});

it("waitForReady resolves after worker-ready message", async () => {
  const client = new WorkerClient();
  const worker = latestWorker();

  const readyPromise = client.waitForReady();
  worker.emitMessage({ type: "worker-ready" });
  await expect(readyPromise).resolves.toBeUndefined();

  client.dispose();
});

it("parseFiles streams layer-result and layer-error, then completes", async () => {
  const client = new WorkerClient();
  const worker = latestWorker();
  worker.emitMessage({ type: "worker-ready" });
  await client.waitForReady();

  const iterator = client.parseFiles([createFile()]);
  const firstNext = iterator.next();
  await flushMicrotasks();

  const request = worker.latestParseRequest();
  expect(request).toBeDefined();
  expect(worker.postedMessages.at(-1)?.transferCount).toBe(1);
  if (request === undefined) {
    throw new Error("Expected parse-request to be posted");
  }

  worker.emitMessage(layerResult(request.requestId));
  const firstValue = expectYielded(await firstNext);
  expect(firstValue.type).toBe("layer-result");

  const secondNext = iterator.next();
  worker.emitMessage({
    type: "layer-error",
    requestId: request.requestId,
    fileName: "board.GTL",
    error: "parse failed",
  });
  const secondValue = expectYielded(await secondNext);
  expect(secondValue.type).toBe("layer-error");

  const doneNext = iterator.next();
  worker.emitMessage({
    ...parseComplete(request.requestId),
    errorCount: 1,
  });
  const done = await doneNext;
  expect(done.done).toBe(true);

  client.dispose();
});

it("cancel posts cancel message for active request", async () => {
  const client = new WorkerClient();
  const worker = latestWorker();
  worker.emitMessage({ type: "worker-ready" });
  await client.waitForReady();

  const iterator = client.parseFiles([createFile()]);
  const nextPromise = iterator.next();
  await flushMicrotasks();

  const request = worker.latestParseRequest();
  if (request === undefined) {
    throw new Error("Expected parse-request to be posted");
  }

  client.cancel();
  expect(worker.hasCancelFor(request.requestId)).toBe(true);
  const done = await nextPromise;
  expect(done.done).toBe(true);

  client.dispose();
});

it("ignores stale messages from another request id", async () => {
  const client = new WorkerClient();
  const worker = latestWorker();
  worker.emitMessage({ type: "worker-ready" });
  await client.waitForReady();

  const iterator = client.parseFiles([createFile()]);
  const nextPromise = iterator.next();
  await flushMicrotasks();

  const request = worker.latestParseRequest();
  if (request === undefined) {
    throw new Error("Expected parse-request to be posted");
  }

  worker.emitMessage(layerResult("stale-request"));
  worker.emitMessage(layerResult(request.requestId));
  const streamed = expectYielded(await nextPromise);
  expect(streamed.type).toBe("layer-result");
  expect(streamed.requestId).toBe(request.requestId);

  const doneNext = iterator.next();
  worker.emitMessage(parseComplete(request.requestId));
  const done = await doneNext;
  expect(done.done).toBe(true);

  client.dispose();
});

it("dispose terminates worker", () => {
  const client = new WorkerClient();
  const worker = latestWorker();

  client.dispose();

  expect(worker.terminated).toBe(true);
});
