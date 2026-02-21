import JSZip from "jszip";
import { ErrorCode, LayerType, type IdentifiedFile } from "../types";
import { identifyLayer } from "./layer-identify";

export const MAX_UNCOMPRESSED_SIZE = 104_857_600; // 100 MB

export class ZipValidationError extends Error {
  readonly code: ErrorCode;
  readonly details: string | undefined;

  constructor(code: ErrorCode, message: string, details?: string) {
    super(message);
    this.name = "ZipValidationError";
    this.code = code;
    this.details = details;
  }
}

function extractBaseName(filePath: string): string {
  const lastSlash = Math.max(filePath.lastIndexOf("/"), filePath.lastIndexOf("\\"));
  return lastSlash >= 0 ? filePath.slice(lastSlash + 1) : filePath;
}

const CONTENT_INSPECT_BYTES = 100;

function identifyByContent(
  fileName: string,
  content: Uint8Array,
): { fileName: string; layerType: LayerType; fileType: "gerber" | "excellon" } | null {
  const inspectLength = Math.min(content.byteLength, CONTENT_INSPECT_BYTES);
  let header = "";
  for (let i = 0; i < inspectLength; i++) {
    const byte = content[i];
    if (byte !== undefined) {
      header += String.fromCharCode(byte);
    }
  }

  if (header.includes("%FSLAX")) {
    return { fileName, layerType: LayerType.Unknown, fileType: "gerber" };
  }

  if (header.includes("M48")) {
    return { fileName, layerType: LayerType.Drill, fileType: "excellon" };
  }

  return null;
}

export async function extractAndIdentify(
  file: File,
  maxUncompressedBytes: number = MAX_UNCOMPRESSED_SIZE,
): Promise<IdentifiedFile[]> {
  let zip: JSZip;
  try {
    const buffer = await file.arrayBuffer();
    zip = await JSZip.loadAsync(buffer);
  } catch {
    throw new ZipValidationError(ErrorCode.InvalidFileType, "File is not a valid ZIP archive");
  }

  const entries = Object.values(zip.files).filter((e) => !e.dir);
  if (entries.length === 0) {
    throw new ZipValidationError(ErrorCode.EmptyZip, "ZIP archive contains no files");
  }

  const results: IdentifiedFile[] = [];
  let totalSize = 0;

  for (const entry of entries) {
    const baseName = extractBaseName(entry.name);
    if (baseName === "" || baseName.startsWith(".")) {
      continue;
    }

    let content: Uint8Array;
    try {
      content = await entry.async("uint8array");
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      if (/encrypt|password/i.test(msg)) {
        throw new ZipValidationError(
          ErrorCode.InvalidFileType,
          "Encrypted ZIP files are not supported",
        );
      }
      throw new ZipValidationError(
        ErrorCode.InvalidFileType,
        `Failed to extract file: ${baseName}`,
        msg,
      );
    }

    totalSize += content.byteLength;
    if (totalSize > maxUncompressedBytes) {
      throw new ZipValidationError(
        ErrorCode.ZipTooLarge,
        `ZIP exceeds ${String(Math.round(maxUncompressedBytes / (1024 * 1024)))} MB uncompressed size limit`,
      );
    }

    let identification = identifyLayer(baseName);

    if (identification.fileType === "unknown") {
      const contentMatch = identifyByContent(baseName, content);
      if (contentMatch) {
        identification = contentMatch;
      }
    }

    if (identification.fileType !== "unknown") {
      results.push({
        fileName: identification.fileName,
        layerType: identification.layerType,
        fileType: identification.fileType,
        content,
      });
    }
  }

  if (results.length === 0) {
    throw new ZipValidationError(
      ErrorCode.NoGerberFiles,
      "ZIP contains no Gerber or Excellon files",
    );
  }

  return results;
}
