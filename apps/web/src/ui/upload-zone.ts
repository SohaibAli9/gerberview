import { LAYER_COLORS } from "../constants";
import type { AppStore } from "../core/store";
import type { WorkerClient } from "../engine/worker-client";
import { extractAndIdentify, ZipValidationError } from "../engine/zip-handler";
import type { Renderer } from "../render/renderer";
import type { SceneManager } from "../scene/scene";
import {
  AppState,
  ErrorCode,
  type AppError,
  type IdentifiedFile,
  type LayerResultMessage,
  type ParsedLayer,
} from "../types";

function isZipFile(file: File): boolean {
  return file.name.toLowerCase().endsWith(".zip");
}

function createHiddenFileInput(): HTMLInputElement {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".zip";
  input.classList.add("hidden");
  input.setAttribute("aria-hidden", "true");
  return input;
}

function clearPreviousState(store: AppStore, sceneManager: SceneManager): void {
  store.layers.value = [];
  store.error.value = null;
  store.loadingProgress.value = null;
  sceneManager.clear();
}

function mapZipError(err: ZipValidationError): AppError {
  switch (err.code) {
    case ErrorCode.EmptyZip:
      return { code: ErrorCode.EmptyZip, message: "The ZIP file is empty." };
    case ErrorCode.NoGerberFiles:
      return {
        code: ErrorCode.NoGerberFiles,
        message: "No Gerber or drill files found in this ZIP.",
      };
    case ErrorCode.ZipTooLarge:
      return { code: ErrorCode.ZipTooLarge, message: err.message };
    case ErrorCode.InvalidFileType:
      return {
        code: ErrorCode.InvalidFileType,
        message: "Please upload a .zip file containing Gerber files.",
      };
    default:
      return { code: err.code, message: err.message };
  }
}

function toStoreParsedLayer(msg: LayerResultMessage): ParsedLayer {
  return {
    id: crypto.randomUUID(),
    fileName: msg.fileName,
    layerType: msg.layerType,
    color: LAYER_COLORS[msg.layerType],
    meta: msg.meta,
    positionBuffer: msg.positions,
    indexBuffer: msg.indices,
    visible: true,
    opacity: 1,
  };
}

function attachDragDropListeners(container: HTMLElement, onFile: (file: File) => void): void {
  let dragCounter = 0;
  const innerBox = container.firstElementChild;

  container.addEventListener("dragenter", (e: DragEvent) => {
    e.preventDefault();
    dragCounter++;
    if (dragCounter === 1 && innerBox !== null) {
      innerBox.classList.add("ring-2", "ring-blue-500");
    }
  });

  container.addEventListener("dragover", (e: DragEvent) => {
    e.preventDefault();
  });

  container.addEventListener("dragleave", () => {
    dragCounter--;
    if (dragCounter === 0 && innerBox !== null) {
      innerBox.classList.remove("ring-2", "ring-blue-500");
    }
  });

  container.addEventListener("drop", (e: DragEvent) => {
    e.preventDefault();
    dragCounter = 0;
    if (innerBox !== null) {
      innerBox.classList.remove("ring-2", "ring-blue-500");
    }

    const file = e.dataTransfer?.files[0];
    if (file === undefined) {
      return;
    }

    onFile(file);
  });
}

function attachFilePickerListeners(
  container: HTMLElement,
  fileInput: HTMLInputElement,
  onFile: (file: File) => void,
): void {
  container.addEventListener("click", (e) => {
    if (e.target === fileInput) {
      return;
    }
    fileInput.click();
  });

  container.addEventListener("keydown", (e: KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      fileInput.click();
    }
  });

  fileInput.addEventListener("change", () => {
    const file = fileInput.files?.[0];
    if (file === undefined) {
      return;
    }
    onFile(file);
    fileInput.value = "";
  });
}

async function handleFile(
  file: File,
  store: AppStore,
  workerClient: WorkerClient,
  sceneManager: SceneManager,
  renderer: Renderer,
): Promise<void> {
  clearPreviousState(store, sceneManager);
  store.appState.value = AppState.Loading;

  let files: IdentifiedFile[];
  try {
    files = await extractAndIdentify(file);
  } catch (err: unknown) {
    if (err instanceof ZipValidationError) {
      store.error.value = mapZipError(err);
    } else {
      store.error.value = {
        code: ErrorCode.ParseFailed,
        message: err instanceof Error ? err.message : "An unexpected error occurred",
      };
    }
    store.appState.value = AppState.Error;
    return;
  }

  const totalFiles = files.length;
  let layersParsed = 0;
  const errors: string[] = [];

  try {
    for await (const msg of workerClient.parseFiles(files)) {
      if (msg.type === "layer-result") {
        const parsedLayer = toStoreParsedLayer(msg);
        store.layers.value = [...store.layers.value, parsedLayer];
        sceneManager.addLayer(parsedLayer);
        layersParsed++;
      } else {
        errors.push(`${msg.fileName}: ${msg.error}`);
      }
      store.loadingProgress.value = {
        current: layersParsed + errors.length,
        total: totalFiles,
        label: `Parsing layer ${String(layersParsed + errors.length)} of ${String(totalFiles)}...`,
      };
    }
  } catch (err: unknown) {
    store.error.value = {
      code: ErrorCode.ParseFailed,
      message: err instanceof Error ? err.message : "An unexpected error occurred during parsing",
    };
    store.appState.value = AppState.Error;
    return;
  }

  if (layersParsed === 0) {
    store.error.value = {
      code: ErrorCode.ParseFailed,
      message: errors.length > 0 ? errors.join("; ") : "No layers could be parsed",
    };
    store.appState.value = AppState.Error;
    return;
  }

  store.loadingProgress.value = null;
  renderer.setBoardBounds(sceneManager.getBounds());
  renderer.setViewState(store.viewState.value);
  renderer.markDirty();
  store.appState.value = AppState.Rendered;
}

/**
 * Initialize the upload zone with drag-drop and file picker behavior.
 * Wires file input to the extraction, parsing, and rendering pipeline.
 */
export function setupUploadZone(
  container: HTMLElement,
  store: AppStore,
  workerClient: WorkerClient,
  sceneManager: SceneManager,
  renderer: Renderer,
): void {
  const fileInput = createHiddenFileInput();
  container.appendChild(fileInput);

  const onFile = (file: File): void => {
    if (!isZipFile(file)) {
      store.error.value = {
        code: ErrorCode.InvalidFileType,
        message: "Please upload a .zip file containing Gerber files.",
      };
      store.appState.value = AppState.Error;
      return;
    }
    void handleFile(file, store, workerClient, sceneManager, renderer);
  };

  attachDragDropListeners(container, onFile);
  attachFilePickerListeners(container, fileInput, onFile);

  store.appState.subscribe((state) => {
    const visible = state === AppState.Empty || state === AppState.Error;
    container.classList.toggle("hidden", !visible);
  });
}
