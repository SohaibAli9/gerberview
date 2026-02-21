import JSZip from "jszip";
import { describe, expect, it } from "vitest";
import { extractAndIdentify, ZipValidationError } from "../engine/zip-handler";

async function zipToFile(zip: JSZip, name = "test.zip"): Promise<File> {
  const data = await zip.generateAsync({ type: "arraybuffer" });
  return new File([data], name, { type: "application/zip" });
}

describe("extractAndIdentify — ZIP validation errors", () => {
  it("BC-ZIP-001: rejects empty file (0 bytes)", async () => {
    const file = new File([], "empty.zip", { type: "application/zip" });
    await expect(extractAndIdentify(file)).rejects.toThrow(ZipValidationError);
    await expect(extractAndIdentify(file)).rejects.toMatchObject({
      code: "INVALID_FILE_TYPE",
    });
  });

  it("BC-ZIP-009: rejects non-ZIP binary", async () => {
    const bytes = new Uint8Array([0xde, 0xad, 0xbe, 0xef, 0x00, 0x01, 0x02, 0x03]);
    const file = new File([bytes], "fake.zip", { type: "application/zip" });
    await expect(extractAndIdentify(file)).rejects.toThrow(ZipValidationError);
    await expect(extractAndIdentify(file)).rejects.toMatchObject({
      code: "INVALID_FILE_TYPE",
    });
  });

  it("BC-ZIP-002: rejects valid ZIP with 0 entries", async () => {
    const zip = new JSZip();
    const file = await zipToFile(zip);
    await expect(extractAndIdentify(file)).rejects.toThrow(ZipValidationError);
    await expect(extractAndIdentify(file)).rejects.toMatchObject({
      code: "EMPTY_ZIP",
    });
  });

  it("BC-ZIP-003: rejects ZIP with only non-Gerber files", async () => {
    const zip = new JSZip();
    zip.file("README.txt", "Hello world");
    zip.file("notes.md", "Some notes");
    const file = await zipToFile(zip);
    await expect(extractAndIdentify(file)).rejects.toThrow(ZipValidationError);
    await expect(extractAndIdentify(file)).rejects.toMatchObject({
      code: "NO_GERBER_FILES",
    });
  });

  it("BC-ZIP-005: rejects ZIP exceeding size limit", async () => {
    const zip = new JSZip();
    zip.file("board.GTL", "x".repeat(100));
    const file = await zipToFile(zip);
    await expect(extractAndIdentify(file, 50)).rejects.toThrow(ZipValidationError);
    await expect(extractAndIdentify(file, 50)).rejects.toMatchObject({
      code: "ZIP_TOO_LARGE",
    });
  });
});

describe("extractAndIdentify — path handling", () => {
  it("BC-ZIP-004: flattens nested directory structure", async () => {
    const zip = new JSZip();
    zip.file("subdir/deep/board.GTL", "%FSLAX36Y36*%");
    const file = await zipToFile(zip);
    const results = await extractAndIdentify(file);

    expect(results).toHaveLength(1);
    expect(results[0]?.fileName).toBe("board.GTL");
    expect(results[0]?.layerType).toBe("top_copper");
  });

  it("BC-ZIP-008: strips path traversal from filenames", async () => {
    const zip = new JSZip();
    zip.file("../../etc/passwd", "not a gerber file");
    zip.file("board.GTL", "%FSLAX36Y36*%");
    const file = await zipToFile(zip);
    const results = await extractAndIdentify(file);

    expect(results).toHaveLength(1);
    expect(results[0]?.fileName).toBe("board.GTL");
    const fileNames = results.map((r) => r.fileName);
    expect(fileNames).not.toContain("passwd");
  });
});

describe("extractAndIdentify — normal operation", () => {
  it("BC-ZIP-006: processes ZIP with many Gerber files", async () => {
    const zip = new JSZip();
    zip.file("board.GTL", "%FSLAX36Y36*%");
    zip.file("board.GBL", "%FSLAX36Y36*%");
    zip.file("board.GTS", "%FSLAX36Y36*%");
    zip.file("board.GBS", "%FSLAX36Y36*%");
    zip.file("board.GTO", "%FSLAX36Y36*%");
    zip.file("board.drl", "M48\nT1C0.8\n%\nX100Y200\nM30");
    const file = await zipToFile(zip);
    const results = await extractAndIdentify(file);

    expect(results).toHaveLength(6);
  });

  it("BC-ZIP-007: handles unicode filenames", async () => {
    const zip = new JSZip();
    zip.file("schaltplan.GTL", "%FSLAX36Y36*%");
    const file = await zipToFile(zip);
    const results = await extractAndIdentify(file);

    expect(results).toHaveLength(1);
    expect(results[0]?.fileName).toBe("schaltplan.GTL");
    expect(results[0]?.layerType).toBe("top_copper");
  });

  it("filters out non-Gerber files in mixed ZIP", async () => {
    const zip = new JSZip();
    zip.file("board.GTL", "%FSLAX36Y36*%");
    zip.file("README.md", "Project docs");
    zip.file("board.drl", "M48\nT1C0.8\n%\nX100Y200\nM30");
    const file = await zipToFile(zip);
    const results = await extractAndIdentify(file);

    expect(results).toHaveLength(2);
    const fileNames = results.map((r) => r.fileName);
    expect(fileNames).toContain("board.GTL");
    expect(fileNames).toContain("board.drl");
    expect(fileNames).not.toContain("README.md");
  });
});

describe("extractAndIdentify — content-based fallback", () => {
  it("identifies unknown extension as Gerber via %FSLAX header", async () => {
    const zip = new JSZip();
    zip.file("unknown.dat", "%FSLAX36Y36*%\n%MOMM*%\nD10*\nX0Y0D03*\nM02*");
    const file = await zipToFile(zip);
    const results = await extractAndIdentify(file);

    expect(results).toHaveLength(1);
    expect(results[0]?.fileType).toBe("gerber");
    expect(results[0]?.fileName).toBe("unknown.dat");
  });

  it("identifies unknown extension as Excellon via M48 header", async () => {
    const zip = new JSZip();
    zip.file("holes.txt", "M48\nMETRIC\nT1C0.80\n%\nT1\nX100Y200\nM30");
    const file = await zipToFile(zip);
    const results = await extractAndIdentify(file);

    expect(results).toHaveLength(1);
    expect(results[0]?.fileType).toBe("excellon");
    expect(results[0]?.layerType).toBe("drill");
  });
});
