import { describe, expect, it } from "vitest";
import { identifyLayer } from "../engine/layer-identify";

describe("identifyLayer — KiCad suffix patterns", () => {
  it("identifies -F_Cu.gbr as top copper", () => {
    const result = identifyLayer("board-F_Cu.gbr");
    expect(result.layerType).toBe("top_copper");
    expect(result.fileType).toBe("gerber");
  });

  it("identifies -B_Cu.gbr as bottom copper", () => {
    const result = identifyLayer("board-B_Cu.gbr");
    expect(result.layerType).toBe("bottom_copper");
    expect(result.fileType).toBe("gerber");
  });

  it("identifies -Edge_Cuts.gbr as board outline", () => {
    const result = identifyLayer("board-Edge_Cuts.gbr");
    expect(result.layerType).toBe("board_outline");
    expect(result.fileType).toBe("gerber");
  });

  it("identifies -F_Paste.gbr as top paste", () => {
    const result = identifyLayer("board-F_Paste.gbr");
    expect(result.layerType).toBe("top_paste");
    expect(result.fileType).toBe("gerber");
  });
});

describe("identifyLayer — Altium/Protel extensions", () => {
  it("identifies .GTL as top copper", () => {
    const result = identifyLayer("board.GTL");
    expect(result.layerType).toBe("top_copper");
    expect(result.fileType).toBe("gerber");
  });

  it("identifies .GBS as bottom solder mask", () => {
    const result = identifyLayer("board.GBS");
    expect(result.layerType).toBe("bottom_solder_mask");
    expect(result.fileType).toBe("gerber");
  });

  it("identifies .GKO as board outline", () => {
    const result = identifyLayer("board.GKO");
    expect(result.layerType).toBe("board_outline");
    expect(result.fileType).toBe("gerber");
  });

  it("identifies .GM1 as board outline", () => {
    const result = identifyLayer("board.GM1");
    expect(result.layerType).toBe("board_outline");
    expect(result.fileType).toBe("gerber");
  });
});

describe("identifyLayer — Eagle extensions", () => {
  it("identifies .cmp as top copper", () => {
    const result = identifyLayer("board.cmp");
    expect(result.layerType).toBe("top_copper");
    expect(result.fileType).toBe("gerber");
  });

  it("identifies .sol as bottom copper", () => {
    const result = identifyLayer("board.sol");
    expect(result.layerType).toBe("bottom_copper");
    expect(result.fileType).toBe("gerber");
  });

  it("identifies .bor as board outline", () => {
    const result = identifyLayer("board.bor");
    expect(result.layerType).toBe("board_outline");
    expect(result.fileType).toBe("gerber");
  });
});

describe("identifyLayer — EasyEDA patterns", () => {
  it("identifies Gerber_TopLayer.GTL as top copper", () => {
    const result = identifyLayer("Gerber_TopLayer.GTL");
    expect(result.layerType).toBe("top_copper");
    expect(result.fileType).toBe("gerber");
  });

  it("identifies Gerber_BottomLayer.GBL as bottom copper", () => {
    const result = identifyLayer("Gerber_BottomLayer.GBL");
    expect(result.layerType).toBe("bottom_copper");
    expect(result.fileType).toBe("gerber");
  });
});

describe("identifyLayer — drill files", () => {
  it("identifies .drl as drill/excellon", () => {
    const result = identifyLayer("board.drl");
    expect(result.layerType).toBe("drill");
    expect(result.fileType).toBe("excellon");
  });

  it("identifies .xln as drill/excellon", () => {
    const result = identifyLayer("board.xln");
    expect(result.layerType).toBe("drill");
    expect(result.fileType).toBe("excellon");
  });
});

describe("identifyLayer — case insensitivity", () => {
  it("handles all uppercase", () => {
    const result = identifyLayer("BOARD.GTL");
    expect(result.layerType).toBe("top_copper");
    expect(result.fileType).toBe("gerber");
  });

  it("handles mixed case", () => {
    const result = identifyLayer("board.GtL");
    expect(result.layerType).toBe("top_copper");
    expect(result.fileType).toBe("gerber");
  });
});

describe("identifyLayer — edge cases", () => {
  it("returns unknown for non-gerber files", () => {
    const result = identifyLayer("README.md");
    expect(result.layerType).toBe("unknown");
    expect(result.fileType).toBe("unknown");
  });

  it("returns unknown for double extensions", () => {
    const result = identifyLayer("board.gbr.bak");
    expect(result.layerType).toBe("unknown");
    expect(result.fileType).toBe("unknown");
  });

  it("returns unknown for files with no extension", () => {
    const result = identifyLayer("noextension");
    expect(result.layerType).toBe("unknown");
    expect(result.fileType).toBe("unknown");
  });

  it("identifies generic .gbr as gerber with unknown layer", () => {
    const result = identifyLayer("board.gbr");
    expect(result.layerType).toBe("unknown");
    expect(result.fileType).toBe("gerber");
  });

  it("strips directory paths with forward slashes", () => {
    const result = identifyLayer("some/path/board.GTL");
    expect(result.fileName).toBe("board.GTL");
    expect(result.layerType).toBe("top_copper");
  });

  it("strips directory paths with backslashes", () => {
    const result = identifyLayer("some\\path\\board.GTL");
    expect(result.fileName).toBe("board.GTL");
    expect(result.layerType).toBe("top_copper");
  });

  it("returns the base filename without path in result", () => {
    const result = identifyLayer("deep/nested/dir/board-F_Cu.gbr");
    expect(result.fileName).toBe("board-F_Cu.gbr");
  });
});
