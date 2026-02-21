import { LayerType, type IdentifiedFile } from "../types";

export type LayerIdentification = Omit<IdentifiedFile, "content">;

interface PatternEntry {
  readonly layerType: LayerType;
  readonly fileType: "gerber" | "excellon";
}

const KICAD_SUFFIX_PATTERNS: ReadonlyMap<string, PatternEntry> = new Map([
  ["-f_cu.gbr", { layerType: LayerType.TopCopper, fileType: "gerber" }],
  ["-b_cu.gbr", { layerType: LayerType.BottomCopper, fileType: "gerber" }],
  ["-f_mask.gbr", { layerType: LayerType.TopSolderMask, fileType: "gerber" }],
  ["-b_mask.gbr", { layerType: LayerType.BottomSolderMask, fileType: "gerber" }],
  ["-f_silks.gbr", { layerType: LayerType.TopSilkscreen, fileType: "gerber" }],
  ["-b_silks.gbr", { layerType: LayerType.BottomSilkscreen, fileType: "gerber" }],
  ["-edge_cuts.gbr", { layerType: LayerType.BoardOutline, fileType: "gerber" }],
  ["-f_paste.gbr", { layerType: LayerType.TopPaste, fileType: "gerber" }],
  ["-b_paste.gbr", { layerType: LayerType.BottomPaste, fileType: "gerber" }],
]);

const EASYEDA_PREFIX_PATTERNS: ReadonlyMap<string, PatternEntry> = new Map([
  ["gerber_toplayer", { layerType: LayerType.TopCopper, fileType: "gerber" }],
  ["gerber_bottomlayer", { layerType: LayerType.BottomCopper, fileType: "gerber" }],
  ["gerber_topsoldermask", { layerType: LayerType.TopSolderMask, fileType: "gerber" }],
  ["gerber_bottomsoldermask", { layerType: LayerType.BottomSolderMask, fileType: "gerber" }],
  ["gerber_topsilkscreen", { layerType: LayerType.TopSilkscreen, fileType: "gerber" }],
  ["gerber_bottomsilkscreen", { layerType: LayerType.BottomSilkscreen, fileType: "gerber" }],
  ["gerber_boardoutline", { layerType: LayerType.BoardOutline, fileType: "gerber" }],
  ["gerber_toppaste", { layerType: LayerType.TopPaste, fileType: "gerber" }],
  ["gerber_bottompaste", { layerType: LayerType.BottomPaste, fileType: "gerber" }],
]);

const EXTENSION_PATTERNS: ReadonlyMap<string, PatternEntry> = new Map([
  // Altium / Protel standard extensions
  [".gtl", { layerType: LayerType.TopCopper, fileType: "gerber" }],
  [".gbl", { layerType: LayerType.BottomCopper, fileType: "gerber" }],
  [".gts", { layerType: LayerType.TopSolderMask, fileType: "gerber" }],
  [".gbs", { layerType: LayerType.BottomSolderMask, fileType: "gerber" }],
  [".gto", { layerType: LayerType.TopSilkscreen, fileType: "gerber" }],
  [".gbo", { layerType: LayerType.BottomSilkscreen, fileType: "gerber" }],
  [".gtp", { layerType: LayerType.TopPaste, fileType: "gerber" }],
  [".gbp", { layerType: LayerType.BottomPaste, fileType: "gerber" }],
  [".gko", { layerType: LayerType.BoardOutline, fileType: "gerber" }],
  [".gm1", { layerType: LayerType.BoardOutline, fileType: "gerber" }],

  // Eagle extensions
  [".cmp", { layerType: LayerType.TopCopper, fileType: "gerber" }],
  [".top", { layerType: LayerType.TopCopper, fileType: "gerber" }],
  [".sol", { layerType: LayerType.BottomCopper, fileType: "gerber" }],
  [".bot", { layerType: LayerType.BottomCopper, fileType: "gerber" }],
  [".stc", { layerType: LayerType.TopSolderMask, fileType: "gerber" }],
  [".tsp", { layerType: LayerType.TopSolderMask, fileType: "gerber" }],
  [".sts", { layerType: LayerType.BottomSolderMask, fileType: "gerber" }],
  [".bsp", { layerType: LayerType.BottomSolderMask, fileType: "gerber" }],
  [".plc", { layerType: LayerType.TopSilkscreen, fileType: "gerber" }],
  [".tsk", { layerType: LayerType.TopSilkscreen, fileType: "gerber" }],
  [".pls", { layerType: LayerType.BottomSilkscreen, fileType: "gerber" }],
  [".bsk", { layerType: LayerType.BottomSilkscreen, fileType: "gerber" }],
  [".bor", { layerType: LayerType.BoardOutline, fileType: "gerber" }],
  [".dim", { layerType: LayerType.BoardOutline, fileType: "gerber" }],

  // Drill file extensions
  [".drl", { layerType: LayerType.Drill, fileType: "excellon" }],
  [".xln", { layerType: LayerType.Drill, fileType: "excellon" }],
  [".drd", { layerType: LayerType.Drill, fileType: "excellon" }],

  // Generic Gerber extensions (layer type unknown without further context)
  [".gbr", { layerType: LayerType.Unknown, fileType: "gerber" }],
  [".ger", { layerType: LayerType.Unknown, fileType: "gerber" }],
  [".pho", { layerType: LayerType.Unknown, fileType: "gerber" }],
]);

function extractBaseName(filePath: string): string {
  const lastSlash = Math.max(filePath.lastIndexOf("/"), filePath.lastIndexOf("\\"));
  return lastSlash >= 0 ? filePath.slice(lastSlash + 1) : filePath;
}

function extractExtension(baseName: string): string {
  const dotIndex = baseName.lastIndexOf(".");
  return dotIndex >= 0 ? baseName.slice(dotIndex) : "";
}

export function identifyLayer(fileName: string): LayerIdentification {
  const baseName = extractBaseName(fileName);
  const lower = baseName.toLowerCase();

  // 1. KiCad suffix patterns (most specific)
  for (const [suffix, entry] of KICAD_SUFFIX_PATTERNS) {
    if (lower.endsWith(suffix)) {
      return { fileName: baseName, layerType: entry.layerType, fileType: entry.fileType };
    }
  }

  // 2. EasyEDA prefix patterns (match base name without extension)
  const extStart = lower.lastIndexOf(".");
  const nameWithoutExt = extStart >= 0 ? lower.slice(0, extStart) : lower;
  for (const [prefix, entry] of EASYEDA_PREFIX_PATTERNS) {
    if (nameWithoutExt.startsWith(prefix)) {
      return { fileName: baseName, layerType: entry.layerType, fileType: entry.fileType };
    }
  }

  // 3. Extension-only patterns (Eagle, Altium, Protel, generic)
  const ext = extractExtension(lower);
  if (ext !== "") {
    const entry = EXTENSION_PATTERNS.get(ext);
    if (entry) {
      return { fileName: baseName, layerType: entry.layerType, fileType: entry.fileType };
    }
  }

  // 4. Fallback
  return { fileName: baseName, layerType: LayerType.Unknown, fileType: "unknown" };
}
