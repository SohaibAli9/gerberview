# GerberView — Requirements & Specifications

> **Document ID:** GVSPEC-001  
> **Version:** 1.0.0  
> **Date:** 2026-02-21  
> **Status:** Draft  
> **Classification:** Internal Engineering  
> **Standards Conformance:** IEEE 830-1998 (SRS), ISO/IEC 25010:2023 (Quality), WCAG 2.1 AA  
> **Upstream Documents:** [Agent Brief](./gerber-viewer-agent-brief.md), [Feasibility Analysis](./gerber-viewer-feasibility.md)

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [System Overview](#2-system-overview)
3. [Technology Stack — Locked Versions](#3-technology-stack--locked-versions)
4. [Project Structure](#4-project-structure)
5. [Functional Requirements](#5-functional-requirements)
6. [Non-Functional Requirements](#6-non-functional-requirements)
7. [Type Specifications](#7-type-specifications)
8. [API Contracts](#8-api-contracts)
9. [Boundary Conditions & Input Validation](#9-boundary-conditions--input-validation)
10. [Error Handling Specification](#10-error-handling-specification)
11. [Coding Standards — Rust](#11-coding-standards--rust)
12. [Coding Standards — TypeScript](#12-coding-standards--typescript)
13. [Coding Standards — GLSL](#13-coding-standards--glsl)
14. [Coding Standards — CSS / Tailwind](#14-coding-standards--css--tailwind)
15. [Linting & Static Analysis](#15-linting--static-analysis)
16. [Testing Specification](#16-testing-specification)
17. [Performance Budgets](#17-performance-budgets)
18. [Accessibility Specification](#18-accessibility-specification)
19. [Browser Compatibility Matrix](#19-browser-compatibility-matrix)
20. [CI/CD Pipeline](#20-cicd-pipeline)
21. [Git Workflow & Conventions](#21-git-workflow--conventions)
22. [Build & Development Workflow](#22-build--development-workflow)
23. [Security Considerations](#23-security-considerations)
24. [Logging & Observability](#24-logging--observability)
25. [Dependency Management](#25-dependency-management)
26. [Glossary](#26-glossary)

---

## 1. Introduction

### 1.1 Purpose

This document is the authoritative specification for the GerberView project. It governs all implementation decisions: code style, type contracts, test requirements, linting rules, boundary conditions, and quality gates. Any code that does not conform to this spec SHALL be rejected in CI and code review.

### 1.2 Scope

GerberView is a static, client-only web application that parses Gerber RS-274X and Excellon drill files in-browser via Rust/WASM and renders them at 60fps via WebGL. No data leaves the browser. No backend exists.

### 1.3 Definitions, Acronyms

See [Glossary](#26-glossary).

### 1.4 Normative Language

The key words "MUST", "MUST NOT", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in [RFC 2119](https://www.ietf.org/rfc/rfc2119.txt).

---

## 2. System Overview

```
┌─── Browser ────────────────────────────────────────────────────┐
│                                                                 │
│  ┌─── TypeScript Layer ─────────────────────────────────────┐   │
│  │  main.ts ──► zip-handler.ts ──► layer-identify.ts        │   │
│  │       │                              │                    │   │
│  │       ▼                              ▼                    │   │
│  │  ui.ts (Tailwind)              WASM bridge calls          │   │
│  │       │                              │                    │   │
│  │       ▼                              ▼                    │   │
│  │  interaction.ts ◄──────── viewer.ts (WebGL)               │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              ▲                                  │
│                    Float32Array / Uint32Array                    │
│                    serde-wasm-bindgen (metadata)                │
│                              │                                  │
│  ┌─── Rust/WASM Layer ──────┴──────────────────────────────┐   │
│  │  lib.rs (wasm_bindgen exports)                           │   │
│  │       │                                                  │   │
│  │       ├── gerber_parser::parse() → GerberDoc             │   │
│  │       ├── geometry/ → vertex buffers                     │   │
│  │       └── excellon/ → drill holes                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Technology Stack — Locked Versions

All dependency versions below are **minimum** versions. Patch-level updates are permitted; minor/major bumps require a spec amendment.

### 3.1 Rust / WASM

| Dependency | Version | Purpose |
|-----------|---------|---------|
| Rust toolchain | stable (MSRV 1.75.0) | Compiler |
| `wasm32-unknown-unknown` | (target) | Compilation target |
| `wasm-pack` | >=0.13.0 | Rust→WASM build tool |
| `gerber_parser` | 0.4.x | Gerber RS-274X parsing |
| `gerber-types` | 0.7.x | (transitive via gerber_parser) |
| `wasm-bindgen` | 0.2.x | Rust↔JS interop |
| `web-sys` | 0.3.x | Browser API bindings |
| `js-sys` | 0.3.x | JS type bindings |
| `serde` | 1.x (features: `derive`) | Serialization framework |
| `serde-wasm-bindgen` | 0.6.x | WASM-native serde adapter |
| `earclip` | 1.8.x | Polygon triangulation |
| `wasm-bindgen-test` | 0.3.x | (dev) WASM test harness |

**Explicitly excluded:** `serde_json` (binary size), `env_logger` feature on `gerber_parser` (WASM-incompatible), `egui`, `gerber-viewer` crate.

### 3.2 TypeScript / Web

| Dependency | Version | Purpose |
|-----------|---------|---------|
| Node.js | >=20 LTS | Runtime |
| TypeScript | >=5.4 | Language |
| Vite | >=6.x | Bundler |
| `vite-plugin-wasm-pack` | latest | WASM integration |
| `vite-plugin-top-level-await` | latest | Async WASM init |
| Tailwind CSS | >=4.x | Styling |
| JSZip | >=3.10 | ZIP extraction |
| Playwright | >=1.48 | E2E testing |
| Vitest | >=2.x | Unit testing |
| ESLint | >=9.x (flat config) | Linting |
| Prettier | >=3.x | Formatting |
| `@axe-core/playwright` | latest | Accessibility testing |
| Turborepo | >=2.x | Monorepo orchestration |
| husky | >=9.x | Git hooks |
| lint-staged | >=15.x | Pre-commit gating |
| `@commitlint/cli` | >=19.x | Commit message linting |
| `@commitlint/config-conventional` | >=19.x | Conventional Commits rules |

### 3.3 CI / Hosting

| Tool | Version / Tier |
|------|---------------|
| GitHub Actions | ubuntu-latest |
| Cloudflare Pages | Free tier |
| `wrangler` CLI | latest |

---

## 4. Project Structure

The project uses a Turborepo monorepo layout. Rust crates live under `crates/`; the web application lives under `apps/web`; shared configuration packages live under `packages/`.

```
gerberview/
├── turbo.json                          # Turborepo pipeline config
├── package.json                        # Workspace root (pnpm workspaces)
├── pnpm-workspace.yaml                 # Workspace definition
├── pnpm-lock.yaml
├── .commitlintrc.json                  # Conventional Commits config
├── .husky/
│   ├── pre-commit                      # lint-staged
│   └── commit-msg                      # commitlint
├── .github/
│   └── workflows/
│       ├── ci.yml                      # Lint + test + build on every push/PR
│       └── deploy.yml                  # Build + deploy to CF Pages on main
├── rustfmt.toml                        # Rust formatting (workspace-level)
├── clippy.toml                         # Clippy configuration
├── deny.toml                           # cargo-deny license/advisory audit
├── Cargo.toml                          # Workspace-level Cargo (virtual manifest)
├── README.md
├── LICENSE                             # MIT
├── .gitignore
│
├── crates/
│   └── gerberview-wasm/                # Rust/WASM crate
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs                  # #[wasm_bindgen] exports only
│       │   ├── geometry/
│       │   │   ├── mod.rs              # pub use re-exports
│       │   │   ├── types.rs            # VertexBuffer, BoundingBox, LayerGeometry
│       │   │   ├── aperture.rs         # Aperture → shape vertices
│       │   │   ├── stroke.rs           # D01 linear draw → quads
│       │   │   ├── arc.rs              # G02/G03 arc tessellation
│       │   │   ├── region.rs           # G36/G37 region → triangulated mesh
│       │   │   ├── polarity.rs         # LPD/LPC handling
│       │   │   ├── macro_eval.rs       # Aperture macro primitive evaluation
│       │   │   └── step_repeat.rs      # SR block duplication
│       │   └── excellon/
│       │       ├── mod.rs
│       │       ├── parser.rs           # Excellon drill file parser
│       │       └── types.rs            # DrillHole, ToolDefinition
│       ├── tests/
│       │   ├── parse_test.rs           # Gerber parsing integration tests
│       │   ├── geometry_test.rs        # Geometry conversion tests
│       │   ├── excellon_test.rs        # Drill file tests
│       │   └── fixtures/               # Real Gerber/Excellon files
│       │       ├── kicad-sample/
│       │       ├── arduino-uno/
│       │       └── eagle-sample/
│       └── benches/
│           └── parse_bench.rs          # criterion benchmarks
│
├── apps/
│   └── web/                            # Frontend application
│       ├── package.json
│       ├── tsconfig.json
│       ├── vite.config.ts
│       ├── tailwind.config.ts
│       ├── postcss.config.js
│       ├── index.html
│       ├── public/
│       │   ├── sw.js                   # Service worker
│       │   └── favicon.svg
│       ├── src/
│       │   ├── main.ts                 # Entry point, WASM init
│       │   ├── types.ts                # All shared TS type definitions
│       │   ├── viewer.ts               # WebGL rendering pipeline
│       │   ├── shaders/
│       │   │   ├── vertex.glsl
│       │   │   └── fragment.glsl
│       │   ├── interaction.ts          # Zoom/pan/touch handlers
│       │   ├── ui.ts                   # Layer panel, upload zone, status bar
│       │   ├── zip-handler.ts          # ZIP extraction + validation
│       │   ├── layer-identify.ts       # Filename → LayerType mapping
│       │   └── constants.ts            # Colors, limits, defaults
│       ├── __tests__/                  # Vitest unit tests
│       │   ├── layer-identify.test.ts
│       │   ├── interaction.test.ts
│       │   ├── zip-handler.test.ts
│       │   └── viewer.test.ts
│       └── e2e/                        # Playwright E2E + visual regression
│           ├── playwright.config.ts
│           ├── fixtures/               # Test ZIP files
│           │   ├── kicad-sample.zip
│           │   ├── eagle-sample.zip
│           │   ├── empty.zip
│           │   ├── invalid.zip
│           │   └── no-gerber.zip
│           └── tests/
│               ├── upload.spec.ts
│               ├── rendering.spec.ts
│               ├── interaction.spec.ts
│               ├── layers.spec.ts
│               ├── error-states.spec.ts
│               └── accessibility.spec.ts
│
└── packages/
    └── eslint-config/                  # Shared ESLint configuration
        ├── package.json
        └── index.js
```

### 4.1 Directory Rules

| Rule | Rationale |
|------|-----------|
| One `mod.rs` per Rust module directory; re-export all public symbols | Flat public API, deep private implementation |
| One `types.ts` per TS package; no type definitions in implementation files | Single source of truth for types |
| Test files mirror source structure (`foo.ts` → `__tests__/foo.test.ts`) | Predictable test location |
| E2E tests live in `e2e/tests/` with `.spec.ts` suffix | Playwright convention |
| Fixture files are committed to the repo (not downloaded at test time) | Deterministic CI |
| No nested `node_modules` — pnpm hoisting handles this | Consistent dependency resolution |

---

## 5. Functional Requirements

### 5.1 File Input

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| FR-100 | Accept `.zip` file via drag-and-drop onto canvas area | MUST | Drop event triggers extraction. Visual feedback on drag-over. |
| FR-101 | Accept `.zip` file via system file picker dialog | MUST | `<input type="file" accept=".zip">` triggered by click. |
| FR-102 | Reject non-ZIP files with descriptive error | MUST | Display: "Please upload a .zip file containing Gerber files." |
| FR-103 | Reject empty ZIP files | MUST | Display: "The ZIP file is empty." |
| FR-104 | Reject ZIP files with no parseable Gerber or Excellon content | MUST | Display: "No Gerber or drill files found in this ZIP." |
| FR-105 | Extract all files from ZIP using JSZip | MUST | Handle flat and nested directory structures. |
| FR-106 | Reject ZIP files exceeding 100 MB uncompressed | MUST | Display size limit error before extraction completes. |
| FR-107 | Support loading a new file while one is already rendered | SHOULD | Previous state is fully cleared before new parse begins. |

### 5.2 Layer Identification

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| FR-200 | Identify layer type from filename patterns | MUST | Support KiCad, Eagle, Altium, EasyEDA, Protel naming conventions. |
| FR-201 | Distinguish Gerber files from Excellon drill files | MUST | By extension first, content-based fallback (`%FSLAX` vs `M48`). |
| FR-202 | Assign "Unknown" type to unrecognized files | MUST | Unknown layers get a neutral gray color and are hidden by default. |
| FR-203 | Support inner copper layers (In1_Cu, In2_Cu, GBL2, etc.) | SHOULD | Assign distinct colors from an extended palette. |

### 5.3 Parsing

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| FR-300 | Parse Gerber RS-274X files via `gerber_parser` crate | MUST | All commands parsed into `GerberDoc`. Partial results on error. |
| FR-301 | Handle aperture definitions: Circle (C), Rectangle (R), Obround (O), Polygon (P) | MUST | All four standard aperture types converted to geometry. |
| FR-302 | Handle D-codes: D01 (draw), D02 (move), D03 (flash) | MUST | Correct state machine behavior. |
| FR-303 | Handle interpolation modes: G01 (linear), G02 (CW arc), G03 (CCW arc) | MUST | G75 (multi-quadrant) mode. |
| FR-304 | Handle regions: G36 (begin), G37 (end) | MUST | Closed polygon, triangulated via `earclip`. |
| FR-305 | Handle polarity: LPD (dark), LPC (clear) | MUST | MVP: clear rendered in background color. |
| FR-306 | Handle step-repeat (SR) | MUST | Geometry duplicated with offsets for each repeat position. |
| FR-307 | Handle aperture macros (AM) — primitives 1, 4, 5, 20, 21 | MUST | Macro primitives evaluated to vertices. |
| FR-308 | Log warning for unsupported features (G74, thermal primitive, deprecated commands) | MUST | Warning in console and UI status bar. |
| FR-309 | Parse Excellon drill files: header, tool definitions, hole coordinates | MUST | Support METRIC/INCH, LZ/TZ suppression, 2.4/2.5 formats. |
| FR-310 | Handle partial parse failures gracefully | MUST | Render parseable layers, show error for failed layers. |

### 5.4 Rendering

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| FR-400 | Render all parsed layers via WebGL on a dark background | MUST | Background: `rgb(26, 26, 26)`. Each layer as colored triangles. |
| FR-401 | Color-code layers by type (see Section 7.4) | MUST | Colors match specification table. |
| FR-402 | Render layers back-to-front with alpha blending | MUST | `gl.blendFunc(SRC_ALPHA, ONE_MINUS_SRC_ALPHA)`. |
| FR-403 | Fit-to-view on initial load | MUST | Bounding box of all layers → view matrix that fits canvas with 5% padding. |
| FR-404 | Maintain aspect ratio during fit-to-view | MUST | No stretching. Board centered in canvas. |

### 5.5 Interaction

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| FR-500 | Zoom via mouse scroll wheel | MUST | Zoom centered on cursor position. Smooth, minimum 10 discrete steps from min to max zoom. |
| FR-501 | Pan via mouse click-and-drag (left button) | MUST | Drag moves the view 1:1 with cursor movement. |
| FR-502 | Fit-to-view button | MUST | Resets to initial computed view. |
| FR-503 | Zoom-in / Zoom-out buttons | MUST | Each click zooms by 1.5x factor. |
| FR-504 | Touch pinch-to-zoom | SHOULD | Two-finger pinch gesture. |
| FR-505 | Touch drag-to-pan (single finger) | SHOULD | Single finger drags after 50ms hold (to avoid scroll conflict). |
| FR-506 | Keyboard: `+`/`=` to zoom in, `-` to zoom out, `0` to fit-to-view | SHOULD | Key events handled when canvas is focused. |

### 5.6 UI Controls

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|-------------------|
| FR-600 | Layer panel: checkbox per layer with color swatch and name | MUST | Toggle visibility. Color matches render color. |
| FR-601 | Layer panel: opacity slider (global) | SHOULD | Range 0.0–1.0, default 1.0. Affects all layer alpha uniformly. |
| FR-602 | Status bar: parsed layer count, shape count, warning count | MUST | Updated after parsing completes. |
| FR-603 | Status bar: cursor coordinate display (mm) | SHOULD | X/Y position in board coordinate space. Updates on mouse move. |
| FR-604 | Status bar: board dimensions (mm) | SHOULD | Width × Height from outline layer bounding box. |
| FR-605 | Loading indicator during parse/geometry conversion | MUST | Spinner + text: "Parsing layer N of M..." |
| FR-606 | Error banner for parse failures | MUST | Red banner, dismissable, with error message. |
| FR-607 | Upload zone: centered prompt when no file loaded | MUST | "Drop Gerber .zip or click to open". Dashed border, hover state. |
| FR-608 | GitHub link in header | MUST | Opens repo in new tab. |

---

## 6. Non-Functional Requirements

### 6.1 Performance

| ID | Requirement | Target | Measurement Method |
|----|-------------|--------|-------------------|
| NFR-100 | Parse + geometry conversion (6-layer board) | < 2,000 ms | `performance.measure()` in browser, Rust criterion bench |
| NFR-101 | Render framerate during zoom/pan | >= 60 fps | `requestAnimationFrame` timing, no frame > 16.67ms |
| NFR-102 | Time to interactive (first meaningful paint) | < 3,000 ms | Lighthouse on 4G throttle |
| NFR-103 | WASM binary (gzipped) | < 800 KB | `wc -c` on gzipped `.wasm` file |
| NFR-104 | Total page weight (gzipped, all assets) | < 1.5 MB | Network tab, production build |
| NFR-105 | Memory usage (6-layer board loaded) | < 128 MB | `performance.memory` (Chrome), WASM heap tracking |

### 6.2 Reliability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-200 | Graceful degradation on malformed Gerber | Show parseable content, warn on errors |
| NFR-201 | WebGL context loss recovery | Re-upload buffers, resume rendering |
| NFR-202 | No `panic!()` in release WASM | All panics caught by wasm-bindgen error handling |

### 6.3 Privacy & Security

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-300 | Zero data leaves the browser | No network requests after initial load |
| NFR-301 | No analytics, tracking, or telemetry | No third-party scripts |
| NFR-302 | Content Security Policy | Strict CSP headers via Cloudflare |

### 6.4 Portability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-400 | Browser support | See [Section 19](#19-browser-compatibility-matrix) |
| NFR-401 | Offline operation after first load | Service worker caches all assets |
| NFR-402 | No server dependency | Fully static — works from `file://` protocol |

### 6.5 Maintainability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-500 | Code coverage (Rust) | >= 90% line coverage |
| NFR-501 | Code coverage (TypeScript) | >= 90% line coverage |
| NFR-502 | Zero lint warnings | All warnings treated as errors in CI |
| NFR-503 | All public Rust APIs documented | `#![deny(missing_docs)]` |
| NFR-504 | All exported TS functions documented | JSDoc on every export |

---

## 7. Type Specifications

All types are strongly typed. No `any` in TypeScript. No raw pointer casts in Rust. No stringly-typed APIs.

### 7.1 Rust Types — Geometry Core

```rust
/// 2D point in board coordinate space (millimeters or inches,
/// determined by the Gerber file's unit specification).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// Output of the geometry pipeline for a single layer.
/// Positions are interleaved [x0, y0, x1, y1, ...].
/// Indices reference into the positions array (triangle list).
#[derive(Debug, Clone)]
pub struct LayerGeometry {
    pub positions: Vec<f32>,
    pub indices: Vec<u32>,
    pub bounds: BoundingBox,
    pub command_count: u32,
    pub vertex_count: u32,
    pub warnings: Vec<String>,
}

/// Metadata returned to JS for a parsed layer.
#[derive(Debug, Clone, Serialize)]
pub struct LayerMeta {
    pub bounds: BoundingBox,
    pub vertex_count: u32,
    pub index_count: u32,
    pub command_count: u32,
    pub warning_count: u32,
    pub warnings: Vec<String>,
}

/// A single drill hole from Excellon parsing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrillHole {
    pub x: f64,
    pub y: f64,
    pub diameter: f64,
}

/// Excellon tool definition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolDefinition {
    pub number: u32,
    pub diameter: f64,
}

/// Result of Excellon parsing for a single file.
#[derive(Debug, Clone)]
pub struct ExcellonResult {
    pub holes: Vec<DrillHole>,
    pub tools: Vec<ToolDefinition>,
    pub units: ExcellonUnits,
}

/// Unit system for Excellon files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExcellonUnits {
    Metric,
    Imperial,
}

/// Polarity state during geometry conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    Dark,
    Clear,
}

/// Interpolation mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationMode {
    Linear,
    ClockwiseArc,
    CounterClockwiseArc,
}
```

### 7.2 Rust Types — Geometry Pipeline State

```rust
/// Mutable state machine that tracks the Gerber interpreter state
/// as commands are processed sequentially.
pub struct GerberState {
    pub current_point: Point,
    pub current_aperture: Option<i32>,
    pub interpolation_mode: InterpolationMode,
    pub polarity: Polarity,
    pub region_mode: bool,
    pub region_points: Vec<Point>,
    pub units: Option<gerber_types::Unit>,
    pub format: Option<gerber_types::CoordinateFormat>,
}
```

### 7.3 TypeScript Types

```typescript
/** Identifies the type of PCB layer. */
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

/** RGBA color with values in [0, 1]. */
export interface LayerColor {
  readonly r: number;
  readonly g: number;
  readonly b: number;
  readonly a: number;
}

/** Axis-aligned bounding box (mirroring Rust BoundingBox). */
export interface BoundingBox {
  readonly minX: number;
  readonly minY: number;
  readonly maxX: number;
  readonly maxY: number;
}

/** Metadata returned from WASM for a parsed layer. */
export interface LayerMeta {
  readonly bounds: BoundingBox;
  readonly vertexCount: number;
  readonly indexCount: number;
  readonly commandCount: number;
  readonly warningCount: number;
  readonly warnings: readonly string[];
}

/** A loaded and parsed layer ready for rendering. */
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

/** Render state for a layer (WebGL handles). */
export interface LayerRenderState {
  readonly positionVBO: WebGLBuffer;
  readonly indexVBO: WebGLBuffer;
  readonly indexCount: number;
}

/** 2D affine view matrix (3x3, column-major for WebGL). */
export type ViewMatrix = readonly [
  number, number, number,
  number, number, number,
  number, number, number,
];

/** All possible application states. */
export const AppState = {
  Empty: "empty",
  Loading: "loading",
  Rendered: "rendered",
  Error: "error",
} as const;

export type AppState = (typeof AppState)[keyof typeof AppState];

/** Error reported to the user. */
export interface AppError {
  readonly code: ErrorCode;
  readonly message: string;
  readonly details?: string;
}

/** Exhaustive error codes. */
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

/** File identification result. */
export interface IdentifiedFile {
  readonly fileName: string;
  readonly layerType: LayerType;
  readonly fileType: "gerber" | "excellon" | "unknown";
  readonly content: Uint8Array;
}

/** Zoom/pan state. */
export interface ViewState {
  centerX: number;
  centerY: number;
  zoom: number;
}

/** Configuration constants. */
export interface ViewerConfig {
  readonly minZoom: number;
  readonly maxZoom: number;
  readonly zoomFactor: number;
  readonly fitPadding: number;
  readonly backgroundColor: readonly [number, number, number, number];
}
```

### 7.4 Layer Color Map

| Layer Type | R | G | B | A | Hex |
|-----------|---|---|---|---|-----|
| `top_copper` | 0.80 | 0.20 | 0.20 | 0.90 | `#CC3333` |
| `bottom_copper` | 0.20 | 0.20 | 0.80 | 0.90 | `#3333CC` |
| `top_solder_mask` | 0.10 | 0.50 | 0.10 | 0.50 | `#1A801A` |
| `bottom_solder_mask` | 0.10 | 0.50 | 0.10 | 0.50 | `#1A801A` |
| `top_silkscreen` | 0.90 | 0.90 | 0.90 | 0.90 | `#E6E6E6` |
| `bottom_silkscreen` | 0.70 | 0.70 | 0.90 | 0.90 | `#B3B3E6` |
| `board_outline` | 0.60 | 0.60 | 0.60 | 1.00 | `#999999` |
| `drill` | 0.90 | 0.90 | 0.20 | 1.00 | `#E6E633` |
| `top_paste` | 0.80 | 0.80 | 0.80 | 0.50 | `#CCCCCC` |
| `bottom_paste` | 0.80 | 0.80 | 0.80 | 0.50 | `#CCCCCC` |
| `inner_copper` | 0.60 | 0.40 | 0.80 | 0.90 | `#9966CC` |
| `unknown` | 0.50 | 0.50 | 0.50 | 0.60 | `#808080` |

This map MUST be defined in `apps/web/src/constants.ts` as a frozen `Record<LayerType, LayerColor>`.

---

## 8. API Contracts

### 8.1 WASM Exports (Rust → JS)

```rust
/// Parse a Gerber RS-274X file from raw bytes.
/// Returns: LayerMeta via serde-wasm-bindgen.
/// Side effect: stores geometry internally, retrievable via get_positions/get_indices.
#[wasm_bindgen]
pub fn parse_gerber(data: &[u8]) -> Result<JsValue, JsValue>;

/// Parse an Excellon drill file from raw bytes.
/// Returns: LayerMeta via serde-wasm-bindgen.
#[wasm_bindgen]
pub fn parse_excellon(data: &[u8]) -> Result<JsValue, JsValue>;

/// Retrieve the position buffer (Float32Array view) for the last parsed layer.
/// CRITICAL: The returned view is invalidated by any subsequent WASM call.
/// The JS side MUST copy it or upload to VBO immediately.
#[wasm_bindgen]
pub fn get_positions() -> Float32Array;

/// Retrieve the index buffer (Uint32Array view) for the last parsed layer.
/// Same invalidation caveat as get_positions().
#[wasm_bindgen]
pub fn get_indices() -> Uint32Array;
```

### 8.2 WASM Export Invariants

1. `parse_gerber` and `parse_excellon` MUST NOT panic. All errors returned as `Err(JsValue)`.
2. `get_positions` MUST return an empty `Float32Array` if called before any parse function.
3. All coordinate values in output buffers MUST be finite (`f32::is_finite()`). NaN and Infinity are forbidden.
4. Index values MUST be valid references into the positions array: `index < positions.len() / 2`.

### 8.3 JS → WASM Call Sequence

```
1. init()                         // WASM module initialization (async)
2. parse_gerber(bytes) → meta     // Parse one layer
3. get_positions() → Float32Array // Copy IMMEDIATELY to VBO
4. get_indices() → Uint32Array    // Copy IMMEDIATELY to VBO
5. Repeat 2-4 for each layer
```

Steps 3 and 4 MUST occur before step 2 is called again for the next layer. Violating this invalidates the typed array views.

---

## 9. Boundary Conditions & Input Validation

Every boundary below MUST have at least one corresponding test case.

### 9.1 ZIP Input Boundaries

| Condition | Expected Behavior | Test ID |
|-----------|-------------------|---------|
| Empty file (0 bytes) | Error: `INVALID_FILE_TYPE` | BC-ZIP-001 |
| Valid ZIP, 0 entries | Error: `EMPTY_ZIP` | BC-ZIP-002 |
| ZIP with only non-Gerber files (e.g., README.md) | Error: `NO_GERBER_FILES` | BC-ZIP-003 |
| ZIP with nested directory structure | Flatten and identify all files | BC-ZIP-004 |
| ZIP > 100 MB uncompressed | Error: `ZIP_TOO_LARGE` | BC-ZIP-005 |
| ZIP with >200 files | Process normally (no artificial limit on file count) | BC-ZIP-006 |
| ZIP with unicode filenames | Handled correctly, filename passed to layer identification | BC-ZIP-007 |
| ZIP with path traversal (`../`) in filenames | Strip path traversal, use basename only | BC-ZIP-008 |
| Non-ZIP binary file with `.zip` extension | Error: `INVALID_FILE_TYPE` | BC-ZIP-009 |
| Encrypted ZIP | Error: "Encrypted ZIP files are not supported" | BC-ZIP-010 |

### 9.2 Gerber Parsing Boundaries

| Condition | Expected Behavior | Test ID |
|-----------|-------------------|---------|
| Empty Gerber file (0 bytes) | Skip with warning: "Empty file" | BC-GBR-001 |
| Gerber file with only whitespace/newlines | Skip with warning | BC-GBR-002 |
| Gerber file with no `%FSLAX` header | Parse best-effort, warn about missing format spec | BC-GBR-003 |
| Gerber with no aperture definitions | Parse, produce zero geometry, warn | BC-GBR-004 |
| Gerber with apertures but no D01/D03 commands | Parse, produce zero geometry (valid: metadata-only file) | BC-GBR-005 |
| Gerber with only D02 moves | Zero geometry, no warning (valid) | BC-GBR-006 |
| Aperture with zero diameter/width | Skip aperture with warning | BC-GBR-007 |
| Aperture with negative dimensions | Absolute value, warn | BC-GBR-008 |
| Coordinate at `(0, 0)` | Valid, render at origin | BC-GBR-009 |
| Very large coordinates (>1,000 mm) | Render, auto-fit view matrix | BC-GBR-010 |
| Negative coordinates | Valid (some boards use negative space) | BC-GBR-011 |
| Arc with zero radius (start == center) | Skip with warning | BC-GBR-012 |
| Arc where start == end with I,J != 0 | Interpret as full circle | BC-GBR-013 |
| Arc where start == end and I,J == 0 | Degenerate, skip | BC-GBR-014 |
| G74 (single-quadrant arc mode) | Log warning, skip arc commands until G75 | BC-GBR-015 |
| Region (G36/G37) with < 3 points | Skip with warning | BC-GBR-016 |
| Region that does not close (last point != first) | Auto-close by connecting last to first | BC-GBR-017 |
| Self-intersecting region | Triangulate best-effort via `earclip` | BC-GBR-018 |
| Nested step-repeat blocks | Flatten: apply outer repeat to inner geometry | BC-GBR-019 |
| Step-repeat with count 0 in X or Y | Skip block, warn | BC-GBR-020 |
| Mixed line endings (CR, LF, CRLF) | `gerber_parser` handles via `BufReader` | BC-GBR-021 |
| Non-UTF8 content | Attempt to parse, error if `gerber_parser` fails | BC-GBR-022 |
| File > 50 MB | Parse with progress reporting | BC-GBR-023 |
| Aperture macro with division by zero | Evaluate to 0, warn | BC-GBR-024 |
| Aperture macro with deeply nested expressions (>10 levels) | Evaluate, warn if >20 levels | BC-GBR-025 |

### 9.3 Excellon Boundaries

| Condition | Expected Behavior | Test ID |
|-----------|-------------------|---------|
| Empty Excellon file | Skip with warning | BC-EXC-001 |
| Header only, no hole data | Parse tools, return zero holes | BC-EXC-002 |
| No header (implicit format) | Default to INCH, 2.4 format, LZ suppression | BC-EXC-003 |
| Tool with zero diameter | Skip tool, warn | BC-EXC-004 |
| Duplicate tool numbers | Last definition wins, warn | BC-EXC-005 |
| Hole coordinate with no prior tool selection | Skip hole, warn | BC-EXC-006 |
| Mixed METRIC/INCH declarations | Use last declaration, warn | BC-EXC-007 |
| Routing commands (G00, G01-G03 in body) | Ignore routing, parse drill holes only | BC-EXC-008 |

### 9.4 WebGL Boundaries

| Condition | Expected Behavior | Test ID |
|-----------|-------------------|---------|
| Canvas size 0x0 (minimized window) | No-op render, resume when canvas visible | BC-GL-001 |
| Canvas resize during render | Update viewport, recalculate view matrix | BC-GL-002 |
| WebGL context lost | Show "WebGL context lost" overlay, attempt restore | BC-GL-003 |
| WebGL context restored | Re-upload all VBOs, resume rendering | BC-GL-004 |
| Layer with zero vertices | Skip draw call (no error) | BC-GL-005 |
| NaN in vertex position buffer | MUST NOT occur (validated in Rust) | BC-GL-006 |
| WebGL not available | Error: `WEBGL_UNAVAILABLE`, show fallback message | BC-GL-007 |
| `OES_element_index_uint` unavailable | Error: "Browser does not support required WebGL extensions" | BC-GL-008 |

### 9.5 Interaction Boundaries

| Condition | Expected Behavior | Test ID |
|-----------|-------------------|---------|
| Zoom to minimum (1/1000x) | Clamp at `minZoom` | BC-INT-001 |
| Zoom to maximum (10000x) | Clamp at `maxZoom` | BC-INT-002 |
| Pan to extreme coordinates (±1e6) | Allow (no artificial pan limits) | BC-INT-003 |
| Rapid scroll events (>60/sec) | Debounce, coalesce into single zoom update | BC-INT-004 |
| Mouse events during loading state | Ignored until `AppState.Rendered` | BC-INT-005 |
| Window resize during interaction | Update canvas dimensions, re-fit if auto-fit was active | BC-INT-006 |

---

## 10. Error Handling Specification

### 10.1 Rust Error Handling

| Rule | Enforcement |
|------|------------|
| All public functions return `Result<T, E>` | `clippy::unwrap_used` = deny |
| No `unwrap()` in library code | `clippy::unwrap_used` = deny |
| No `expect()` in library code | `clippy::expect_used` = deny |
| `panic!()` forbidden in library code | `clippy::panic` = deny |
| Use `thiserror` for domain error types | Manual review |
| Errors crossing WASM boundary are converted to `JsValue` via `map_err` | Compile-time enforcement via return types |
| Internal errors preserve context via `anyhow::Context` in test code only | Allowed in `#[cfg(test)]` only |

### 10.2 TypeScript Error Handling

| Rule | Enforcement |
|------|------------|
| No uncaught promises | ESLint: `@typescript-eslint/no-floating-promises` = error |
| All `catch` blocks must type-narrow the error | ESLint: `@typescript-eslint/no-unsafe-catch` custom rule |
| WASM calls wrapped in try/catch | Manual review + E2E test coverage |
| User-facing errors use `AppError` type | TypeScript compiler (type check) |
| No `throw` of string literals | ESLint: `@typescript-eslint/only-throw-error` = error |

### 10.3 Error Propagation Flow

```
Rust parse error
  → thiserror enum variant
  → map to JsValue string (in lib.rs)
  → caught by JS try/catch
  → mapped to AppError { code, message, details }
  → displayed in error banner (ui.ts)
  → logged to console with full details
```

---

## 11. Coding Standards — Rust

### 11.1 Crate-Level Attributes

Every `.rs` file in `crates/gerberview-wasm/src/` MUST be governed by these crate-level attributes in `lib.rs`:

```rust
#![deny(warnings)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::indexing_slicing)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
```

### 11.2 `rustfmt.toml`

```toml
edition = "2021"
max_width = 100
tab_spaces = 4
use_field_init_shorthand = true
use_try_shorthand = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
reorder_imports = true
reorder_modules = true
newline_style = "Unix"
```

### 11.3 `clippy.toml`

```toml
msrv = "1.75.0"
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 7
type-complexity-threshold = 250
```

### 11.4 Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Crate names | `snake_case` | `gerberview_wasm` |
| Module names | `snake_case` | `macro_eval` |
| Types (struct, enum, trait) | `PascalCase` | `LayerGeometry` |
| Functions, methods | `snake_case` | `expand_aperture` |
| Constants | `SCREAMING_SNAKE_CASE` | `MAX_ARC_SEGMENTS` |
| Type parameters | Single uppercase letter or short `PascalCase` | `T`, `R: Read` |
| Lifetime parameters | Short lowercase | `'a`, `'buf` |
| Enum variants | `PascalCase` | `Polarity::Dark` |
| Macro names | `snake_case!` | N/A (no custom macros expected) |

### 11.5 Documentation

| Item | Requirement |
|------|------------|
| All `pub` items | `///` doc comment required (`#![deny(missing_docs)]`) |
| Module-level docs | `//!` at top of each module file |
| Complex algorithms | Block comments explaining the math/approach |
| Unsafe blocks | Forbidden (`#![deny(unsafe_code)]`) |
| TODO/FIXME | Linked to a GitHub issue number: `// TODO(#42): description` |

### 11.6 Import Ordering

Enforced by `rustfmt` `group_imports = "StdExternalCrate"`:

```rust
// 1. std library
use std::io::{BufReader, Cursor};

// 2. External crates
use gerber_types::Command;
use serde::Serialize;

// 3. Crate-internal
use crate::geometry::types::LayerGeometry;
```

### 11.7 Numeric Precision

| Rule | Detail |
|------|--------|
| All board-space coordinates | `f64` (sufficient for sub-micrometer PCB precision) |
| Vertex buffer output | `f32` (GPU precision, reduces memory and transfer size) |
| Conversion `f64 → f32` | Explicit cast with `as f32`, acceptable precision loss for rendering |
| Floating-point comparison | Use `(a - b).abs() < epsilon` with `epsilon = 1e-9` for f64, `1e-6` for f32 |
| No integer overflow in index calculations | Use `u32` for indices; check `positions.len() / 2 <= u32::MAX` |

---

## 12. Coding Standards — TypeScript

### 12.1 `tsconfig.json`

```jsonc
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true,
    "noPropertyAccessFromIndexSignature": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "forceConsistentCasingInFileNames": true,
    "isolatedModules": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "skipLibCheck": true,
    "outDir": "dist",
    "rootDir": "src"
  },
  "include": ["src/**/*.ts"],
  "exclude": ["node_modules", "dist", "e2e"]
}
```

### 12.2 ESLint Configuration (Flat Config)

```js
// packages/eslint-config/index.js
import eslint from "@eslint/js";
import tseslint from "typescript-eslint";
import prettier from "eslint-config-prettier";

export default tseslint.config(
  eslint.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  prettier,
  {
    rules: {
      // --- Type safety (all errors, not warnings) ---
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-unsafe-assignment": "error",
      "@typescript-eslint/no-unsafe-call": "error",
      "@typescript-eslint/no-unsafe-member-access": "error",
      "@typescript-eslint/no-unsafe-return": "error",
      "@typescript-eslint/no-unsafe-argument": "error",
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/no-misused-promises": "error",
      "@typescript-eslint/await-thenable": "error",
      "@typescript-eslint/only-throw-error": "error",
      "@typescript-eslint/prefer-promise-reject-errors": "error",

      // --- Strict style ---
      "@typescript-eslint/explicit-function-return-type": ["error", {
        allowExpressions: true,
        allowTypedFunctionExpressions: true,
      }],
      "@typescript-eslint/explicit-module-boundary-types": "error",
      "@typescript-eslint/consistent-type-imports": ["error", {
        prefer: "type-imports",
      }],
      "@typescript-eslint/consistent-type-exports": "error",
      "@typescript-eslint/no-import-type-side-effects": "error",
      "@typescript-eslint/no-non-null-assertion": "error",
      "@typescript-eslint/strict-boolean-expressions": "error",

      // --- Code quality ---
      "no-console": ["error", { allow: ["warn", "error", "group",
        "groupEnd", "time", "timeEnd"] }],
      "no-debugger": "error",
      "no-alert": "error",
      "eqeqeq": ["error", "always"],
      "prefer-const": "error",
      "no-var": "error",
      "no-param-reassign": "error",
      "no-nested-ternary": "error",
      "complexity": ["error", 20],
      "max-depth": ["error", 4],
      "max-lines-per-function": ["error", { max: 100, skipBlankLines: true,
        skipComments: true }],
    },
  }
);
```

### 12.3 Prettier Configuration

```jsonc
// .prettierrc (root)
{
  "semi": true,
  "singleQuote": false,
  "trailingComma": "all",
  "printWidth": 100,
  "tabWidth": 2,
  "useTabs": false,
  "bracketSpacing": true,
  "arrowParens": "always",
  "endOfLine": "lf"
}
```

### 12.4 Naming Conventions

| Item | Convention | Example |
|------|-----------|---------|
| Files | `kebab-case.ts` | `layer-identify.ts` |
| Test files | `kebab-case.test.ts` | `layer-identify.test.ts` |
| E2E specs | `kebab-case.spec.ts` | `upload.spec.ts` |
| Interfaces | `PascalCase` (no `I` prefix) | `ParsedLayer` |
| Type aliases | `PascalCase` | `ViewMatrix` |
| Constants (module-level) | `PascalCase` for const objects, `SCREAMING_SNAKE_CASE` for primitives | `LayerType`, `MAX_ZOOM` |
| Functions | `camelCase` | `identifyLayer` |
| Variables | `camelCase` | `currentZoom` |
| Enum-like const objects | `PascalCase` | `AppState` |
| CSS classes | Tailwind utility classes (no custom class names except for JS hooks prefixed with `js-`) | `js-upload-zone` |

### 12.5 Import Ordering

Enforced by ESLint `import/order` (or `@typescript-eslint/consistent-type-imports`):

```typescript
// 1. Node built-ins (none expected in browser code)
// 2. External packages
import JSZip from "jszip";

// 3. WASM module
import init, { parse_gerber, get_positions } from "gerberview-wasm";

// 4. Internal modules (absolute paths within app)
import { identifyLayer } from "./layer-identify";
import type { ParsedLayer, LayerType } from "./types";

// 5. Type-only imports last (enforced by consistent-type-imports)
```

---

## 13. Coding Standards — GLSL

### 13.1 File Conventions

| Rule | Detail |
|------|--------|
| File extension | `.glsl` |
| Location | `apps/web/src/shaders/` |
| Naming | `vertex.glsl`, `fragment.glsl` |
| Precision | `precision mediump float;` in fragment shader |
| Uniforms | Prefixed with `u_` | `u_viewMatrix`, `u_color` |
| Attributes | Prefixed with `a_` | `a_position` |
| Varyings | Prefixed with `v_` (if needed in future) |
| No magic numbers | Use `#define` or uniform for all tunable values |

### 13.2 Shader Source

Shaders are imported as raw strings via Vite's `?raw` suffix:

```typescript
import vertexSource from "./shaders/vertex.glsl?raw";
import fragmentSource from "./shaders/fragment.glsl?raw";
```

---

## 14. Coding Standards — CSS / Tailwind

### 14.1 Tailwind Configuration

```typescript
// apps/web/tailwind.config.ts
import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,js}"],
  theme: {
    extend: {
      colors: {
        board: {
          bg: "#1a1a1a",
          panel: "#242424",
          border: "#3a3a3a",
        },
      },
      fontFamily: {
        mono: ['"JetBrains Mono"', '"Fira Code"', "monospace"],
      },
    },
  },
  plugins: [],
} satisfies Config;
```

### 14.2 CSS Rules

| Rule | Detail |
|------|--------|
| No custom CSS classes except for JS hooks (`js-*` prefix) | Use Tailwind utilities exclusively |
| `@apply` forbidden | Compose utilities in HTML/TS, not in CSS files |
| `main.css` contains only: `@tailwind base; @tailwind components; @tailwind utilities;` and CSS custom properties for theming | Minimal CSS surface |
| No `!important` | Use Tailwind specificity or restructure |
| Dark theme is the only theme | No theme switching (dark background is the default for PCB viewers) |

---

## 15. Linting & Static Analysis

### 15.1 Warning-as-Error Policy

**Every warning is an error.** No exceptions. This applies to all tools in all environments (local dev, CI, pre-commit hooks).

| Tool | Flag / Rule | Scope |
|------|------------|-------|
| `rustc` | `#![deny(warnings)]` | All Rust source |
| `clippy` | `-- -D warnings` | CI + pre-commit |
| `clippy` | `#![deny(clippy::all, clippy::pedantic, clippy::nursery)]` | Crate-level |
| TypeScript | `strict: true` + all extra strict flags | tsconfig.json |
| ESLint | All rules set to `"error"`, none to `"warn"` | eslint.config.js |
| Prettier | `--check` in CI, auto-fix in pre-commit | All TS/JS/JSON |
| `cargo fmt` | `--check` in CI | All Rust source |

### 15.2 `cargo-deny` (Dependency Audit)

```toml
# deny.toml
[licenses]
allow = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib"]
deny = ["GPL-2.0", "GPL-3.0", "AGPL-3.0"]
copyleft = "deny"
confidence-threshold = 0.8

[advisories]
db-path = "~/.cargo/advisory-db"
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"

[bans]
multiple-versions = "warn"
deny = []

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

### 15.3 Pre-Commit Hook Checks

Executed by `husky` + `lint-staged` on every commit:

```jsonc
// package.json (root)
{
  "lint-staged": {
    "*.{ts,js}": ["eslint --fix", "prettier --write"],
    "*.{json,md,yaml,yml}": ["prettier --write"],
    "*.rs": ["rustfmt --edition 2021"]
  }
}
```

The `commit-msg` hook runs `commitlint` to enforce Conventional Commits format.

### 15.4 Static Analysis Summary

| Layer | Tool | When | Severity |
|-------|------|------|----------|
| Rust lint | clippy (pedantic + nursery) | Pre-commit + CI | Error |
| Rust format | rustfmt | Pre-commit + CI | Error |
| Rust dependencies | cargo-deny | CI only | Error (vuln/license), Warn (unmaintained) |
| Rust WASM | `cargo build --target wasm32` | CI | Error |
| TS lint | ESLint strict + type-checked | Pre-commit + CI | Error |
| TS types | `tsc --noEmit` | CI | Error |
| TS format | Prettier | Pre-commit + CI | Error |
| Commit message | commitlint | commit-msg hook | Error |
| Accessibility | axe-core via Playwright | CI (E2E) | Error |
| Bundle size | Custom script | CI | Error if > threshold |

---

## 16. Testing Specification

### 16.1 Test Pyramid

```
              ┌──────────────────┐
              │    E2E Tests     │  ← Playwright (visual, interaction, a11y)
              │   (6-10 specs)   │
            ┌─┴──────────────────┴─┐
            │  Integration Tests   │  ← Rust: real Gerber → geometry
            │    (15-25 tests)     │     TS: WASM → render pipeline
          ┌─┴──────────────────────┴─┐
          │      Unit Tests          │  ← Rust: geometry math, parsing
          │     (80-120 tests)       │     TS: layer ID, view math, utils
          └──────────────────────────┘
```

### 16.2 Coverage Requirements

| Layer | Minimum Line Coverage | Minimum Branch Coverage | Tool |
|-------|----------------------|------------------------|------|
| Rust (`crates/gerberview-wasm/src/`) | 90% | 80% | `cargo-tarpaulin` or `llvm-cov` |
| TypeScript (`apps/web/src/`) | 90% | 80% | Vitest + `@vitest/coverage-v8` |
| E2E | N/A (not measured by line) | N/A | Playwright |

Coverage gates are enforced in CI. A PR that drops coverage below the threshold MUST NOT merge.

### 16.3 Rust Unit Tests

Located in the same file as the implementation (`#[cfg(test)]` modules) or in `crates/gerberview-wasm/tests/`.

#### 16.3.1 Geometry — Aperture

| Test ID | Description | Input | Expected Output |
|---------|-------------|-------|----------------|
| UT-APR-001 | Circle aperture → N-gon vertices | Circle d=1.0, center=(0,0), N=32 | 32 vertices on unit circle |
| UT-APR-002 | Circle aperture vertex distance from center | Circle d=2.0, center=(5,3) | All vertices at distance 1.0 ± 1e-6 from (5,3) |
| UT-APR-003 | Rectangle aperture → 4 vertices | Rect 2.0×1.0, center=(0,0) | Corners at (±1.0, ±0.5) |
| UT-APR-004 | Rectangle aperture → 2 triangles | Rect any | 6 indices forming 2 CCW triangles |
| UT-APR-005 | Obround horizontal → rect + 2 semicircles | 3.0×1.0, center=(0,0) | Width > height: semicircles on left/right |
| UT-APR-006 | Obround vertical → rect + 2 semicircles | 1.0×3.0, center=(0,0) | Height > width: semicircles on top/bottom |
| UT-APR-007 | Polygon aperture → N-gon with rotation | 6 sides, d=2.0, rot=30° | 6 vertices, first vertex at 30° |
| UT-APR-008 | Zero-diameter circle | Circle d=0.0 | Empty geometry + warning |
| UT-APR-009 | Negative-dimension rectangle | Rect -2.0×-1.0 | Absolute values used, warning |

#### 16.3.2 Geometry — Stroke

| Test ID | Description | Input | Expected Output |
|---------|-------------|-------|----------------|
| UT-STR-001 | Horizontal line → quad | (0,0)→(10,0), width=2.0 | 4 vertices: (0,±1), (10,±1) |
| UT-STR-002 | Vertical line → quad | (0,0)→(0,10), width=2.0 | 4 vertices: (±1,0), (±1,10) |
| UT-STR-003 | Diagonal line → rotated quad | (0,0)→(3,4), width=2.0 | Quad perpendicular to line direction |
| UT-STR-004 | Zero-length line | (5,5)→(5,5), width=1.0 | Degenerate: single circle flash (or skip + warn) |
| UT-STR-005 | Line with circular endcaps | (0,0)→(10,0), circle aperture d=2.0 | Quad + 2 semicircles (N vertices each) |
| UT-STR-006 | Line with square endcaps | (0,0)→(10,0), rect aperture | Quad only, no semicircles |

#### 16.3.3 Geometry — Arc

| Test ID | Description | Input | Expected Output |
|---------|-------------|-------|----------------|
| UT-ARC-001 | 90° CW arc → tessellated segments | Quarter circle, radius=5 | N points on arc within 1e-4 of radius |
| UT-ARC-002 | 90° CCW arc | Quarter circle, radius=5 | Points in opposite winding |
| UT-ARC-003 | 180° arc | Semicircle | Points span half circle |
| UT-ARC-004 | 360° arc (full circle) | start==end, nonzero I,J | Complete circle of points |
| UT-ARC-005 | Very small arc (<1°) | Near-zero sweep | At least 2 segments |
| UT-ARC-006 | Arc with stroke widening | Arc + aperture d=1.0 | Quad strip along arc path |
| UT-ARC-007 | Zero-radius arc | I=0, J=0 | Skip with warning |

#### 16.3.4 Geometry — Region

| Test ID | Description | Input | Expected Output |
|---------|-------------|-------|----------------|
| UT-REG-001 | Square region → 2 triangles | 4-point square | 2 triangles, 4 vertices |
| UT-REG-002 | L-shaped region | 6-point L-shape | Correct triangulation (>=4 triangles) |
| UT-REG-003 | Triangle region | 3-point triangle | 1 triangle, 3 vertices |
| UT-REG-004 | Concave polygon | Arrow shape | Valid triangulation (no inverted triangles) |
| UT-REG-005 | Region with 2 points | Degenerate | Skip with warning |
| UT-REG-006 | Region with 1 point | Degenerate | Skip with warning |
| UT-REG-007 | Self-intersecting polygon | Bowtie shape | earclip handles, verify non-empty output |
| UT-REG-008 | Region with arc boundary | Mixed linear + arc segments | Arcs tessellated before triangulation |

#### 16.3.5 Excellon Parser

| Test ID | Description | Input | Expected Output |
|---------|-------------|-------|----------------|
| UT-EXC-001 | Simple drill file | M48, 2 tools, 5 holes | 2 tools, 5 holes at correct positions |
| UT-EXC-002 | Metric units | METRIC header | Coordinates in mm |
| UT-EXC-003 | Imperial units | INCH header | Coordinates in inches |
| UT-EXC-004 | Leading zero suppression | LZ format | Correct coordinate parsing |
| UT-EXC-005 | Trailing zero suppression | TZ format | Correct coordinate parsing |
| UT-EXC-006 | Tool change mid-file | T1 ... T2 ... | Holes assigned correct diameters |

#### 16.3.6 Polarity, Step-Repeat, Macro

| Test ID | Description |
|---------|-------------|
| UT-POL-001 | Dark polarity → normal geometry |
| UT-POL-002 | Clear polarity → geometry with background color flag |
| UT-POL-003 | Polarity switch mid-layer |
| UT-SR-001 | 2x3 step-repeat → 6 copies of geometry |
| UT-SR-002 | Step-repeat with spacing |
| UT-SR-003 | Step-repeat with zero count in X → skip + warn |
| UT-MAC-001 | Circle primitive → vertices |
| UT-MAC-002 | Vector line primitive → vertices |
| UT-MAC-003 | Outline primitive → polygon |
| UT-MAC-004 | Exposure off → clear geometry |
| UT-MAC-005 | Arithmetic expression evaluation ($1×2+$2) |

### 16.4 Rust Integration Tests

Located in `crates/gerberview-wasm/tests/`.

| Test ID | Description | Fixture |
|---------|-------------|---------|
| IT-001 | Parse KiCad Gerber → non-empty command list | `fixtures/kicad-sample/*.gbr` |
| IT-002 | Parse → geometry → vertex count > 0 for copper layer | `fixtures/kicad-sample/*-F_Cu.gbr` |
| IT-003 | Parse → geometry → bounds within expected range | `fixtures/arduino-uno/*.GTL` |
| IT-004 | Parse all layers in ZIP → all identified correctly | `fixtures/kicad-sample/` |
| IT-005 | Parse Eagle board → compatible output | `fixtures/eagle-sample/` |
| IT-006 | Large file parse time < 2000ms | Complex board fixture |
| IT-007 | Parse malformed file → partial result + error, no panic | Intentionally corrupted file |

### 16.5 TypeScript Unit Tests (Vitest)

Located in `apps/web/__tests__/`.

| Test ID | Description |
|---------|-------------|
| UT-TS-001 | `identifyLayer("board-F_Cu.gbr")` → `{ type: "top_copper", fileType: "gerber" }` |
| UT-TS-002 | `identifyLayer("board.GTL")` → `{ type: "top_copper", fileType: "gerber" }` |
| UT-TS-003 | `identifyLayer("board.cmp")` → `{ type: "top_copper", fileType: "gerber" }` (Eagle) |
| UT-TS-004 | `identifyLayer("board.drl")` → `{ type: "drill", fileType: "excellon" }` |
| UT-TS-005 | `identifyLayer("README.md")` → `{ type: "unknown", fileType: "unknown" }` |
| UT-TS-006 | `identifyLayer("Gerber_TopLayer.GTL")` → top copper (EasyEDA) |
| UT-TS-007 | Case-insensitive matching: `"BOARD.GTL"` → top copper |
| UT-TS-008 | View matrix identity → no transform |
| UT-TS-009 | Zoom centered on cursor → correct matrix |
| UT-TS-010 | Pan by (dx, dy) → correct translation |
| UT-TS-011 | Fit-to-view → board fills canvas with padding |
| UT-TS-012 | ZIP validation: empty ZIP rejected |
| UT-TS-013 | ZIP validation: non-ZIP binary rejected |
| UT-TS-014 | ZIP validation: >100MB rejected |

### 16.6 E2E Tests (Playwright)

Located in `apps/web/e2e/tests/`.

#### 16.6.1 Configuration

```typescript
// apps/web/e2e/playwright.config.ts
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env["CI"],
  retries: process.env["CI"] ? 2 : 0,
  workers: process.env["CI"] ? 1 : undefined,
  reporter: [
    ["html"],
    ["json", { outputFile: "test-results.json" }],
  ],
  use: {
    baseURL: "http://localhost:5173",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
  webServer: {
    command: "pnpm run dev",
    port: 5173,
    reuseExistingServer: !process.env["CI"],
  },
});
```

#### 16.6.2 E2E Test Specs

| Spec File | Test Cases |
|-----------|-----------|
| `upload.spec.ts` | File picker upload, drag-drop upload, invalid file rejection, empty ZIP rejection, re-upload replaces previous |
| `rendering.spec.ts` | Board renders after upload, all layers visible by default, correct colors per layer type, visual regression snapshot |
| `interaction.spec.ts` | Scroll zoom changes view, click-drag pans, fit-to-view button resets, zoom buttons work, keyboard shortcuts |
| `layers.spec.ts` | Toggle layer hides/shows it, opacity slider changes alpha, layer panel shows correct names |
| `error-states.spec.ts` | No-WebGL fallback message, malformed Gerber shows error + partial render, WASM load failure |
| `accessibility.spec.ts` | axe-core scan passes WCAG 2.1 AA, keyboard navigation through all controls, focus visible indicators |

#### 16.6.3 Visual Regression

Playwright screenshot comparisons with a tolerance threshold:

```typescript
await expect(page.locator("canvas")).toHaveScreenshot("kicad-board-all-layers.png", {
  maxDiffPixelRatio: 0.01,
});
```

Reference screenshots are committed to `e2e/tests/__screenshots__/` and updated explicitly via `npx playwright test --update-snapshots`.

### 16.7 Performance Tests

| Test ID | What | Target | Tool |
|---------|------|--------|------|
| PERF-001 | Gerber parse time (KiCad sample) | < 500 ms | Rust `criterion` bench |
| PERF-002 | Geometry conversion (KiCad sample) | < 1000 ms | Rust `criterion` bench |
| PERF-003 | Full pipeline: upload → first render | < 2000 ms | Playwright `performance.measure` |
| PERF-004 | Frame time during zoom/pan | < 16.67 ms (60fps) | `requestAnimationFrame` delta |
| PERF-005 | WASM binary size (gzipped) | < 800 KB | CI script: `gzip -c file.wasm \| wc -c` |
| PERF-006 | Total bundle size (gzipped) | < 1.5 MB | CI script on dist/ |

### 16.8 Accessibility Tests

Integrated into E2E via `@axe-core/playwright`:

```typescript
import AxeBuilder from "@axe-core/playwright";

test("should pass WCAG 2.1 AA", async ({ page }) => {
  await page.goto("/");
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
    .analyze();
  expect(results.violations).toEqual([]);
});
```

### 16.9 Test Execution Summary

| Command | What it runs | When |
|---------|-------------|------|
| `cargo test` | Rust unit + integration tests | Pre-commit (via `cargo test` in lint-staged for `.rs`), CI |
| `pnpm run test` | Vitest unit tests | CI |
| `pnpm run test:e2e` | Playwright E2E + visual regression + a11y | CI |
| `pnpm run test:coverage` | Vitest with coverage report | CI (gated) |
| `cargo tarpaulin` | Rust coverage report | CI (gated) |
| `cargo bench` | Criterion benchmarks | Manual + CI (non-blocking) |

---

## 17. Performance Budgets

Enforced in CI via custom scripts or Lighthouse CI.

| Metric | Budget | Action if exceeded |
|--------|--------|-------------------|
| WASM binary (gzipped) | 800 KB | CI fails |
| Total JS bundle (gzipped) | 300 KB | CI fails |
| Total page weight (gzipped) | 1.5 MB | CI fails |
| Lighthouse Performance score | >= 90 | CI warns |
| Largest Contentful Paint | < 2.5s | CI warns |
| Cumulative Layout Shift | < 0.1 | CI warns |
| Parse + render (6-layer board) | < 2,000 ms | Benchmark regression CI fails |
| Render frame time | < 16.67 ms | Manual testing |

---

## 18. Accessibility Specification

### 18.1 WCAG 2.1 AA Compliance

| Criterion | Requirement | Implementation |
|-----------|------------|---------------|
| 1.1.1 Non-text Content | Canvas has `aria-label` describing content | `<canvas aria-label="PCB Gerber layer visualization">` |
| 1.3.1 Info and Relationships | Layer panel uses semantic HTML | `<fieldset>`, `<legend>`, `<label>` |
| 1.4.3 Contrast (Minimum) | 4.5:1 contrast ratio for text | Dark theme verified via axe-core |
| 1.4.11 Non-text Contrast | 3:1 for UI controls | Button borders, checkbox states |
| 2.1.1 Keyboard | All controls keyboard-accessible | Tab order: upload → layer panel → zoom buttons → canvas |
| 2.1.2 No Keyboard Trap | Focus can leave canvas | `Escape` key releases canvas focus |
| 2.4.1 Bypass Blocks | Skip-to-content link | `<a href="#main" class="sr-only focus:not-sr-only">` |
| 2.4.3 Focus Order | Logical tab sequence | DOM order matches visual order |
| 2.4.7 Focus Visible | Visible focus indicator | `ring-2 ring-blue-500` Tailwind utilities |
| 3.1.1 Language of Page | `<html lang="en">` | In `index.html` |
| 4.1.2 Name, Role, Value | ARIA attributes on custom controls | `role="slider"` for opacity, `aria-checked` for toggles |

### 18.2 Keyboard Navigation Map

| Key | Action | Context |
|-----|--------|---------|
| `Tab` | Move focus to next control | Global |
| `Shift+Tab` | Move focus to previous control | Global |
| `Space` / `Enter` | Activate focused button / toggle checkbox | Buttons, checkboxes |
| `+` / `=` | Zoom in | Canvas focused |
| `-` | Zoom out | Canvas focused |
| `0` | Fit to view | Canvas focused |
| `Arrow keys` | Pan (10px per press) | Canvas focused |
| `Escape` | Release canvas focus / dismiss error | Canvas, error banner |

### 18.3 Reduced Motion

```css
@media (prefers-reduced-motion: reduce) {
  * { transition-duration: 0.01ms !important; animation-duration: 0.01ms !important; }
}
```

---

## 19. Browser Compatibility Matrix

| Browser | Minimum Version | WebGL | WASM | Status |
|---------|----------------|-------|------|--------|
| Chrome | 90+ | 1.0 | Yes | Primary target |
| Firefox | 90+ | 1.0 | Yes | Primary target |
| Safari | 15+ | 1.0 | Yes | Primary target |
| Edge | 90+ | 1.0 | Yes | Primary target |
| Chrome Android | 90+ | 1.0 | Yes | Secondary (functional) |
| Safari iOS | 15+ | 1.0 | Yes | Secondary (functional) |
| Samsung Internet | 15+ | 1.0 | Yes | Best-effort |

**Not supported:** IE11, Opera Mini, UC Browser.

**Required WebGL extensions:**
- `OES_element_index_uint` (universally available)

---

## 20. CI/CD Pipeline

### 20.1 CI Workflow (`.github/workflows/ci.yml`)

Triggers: push to any branch, all PRs.

```yaml
# Conceptual steps (not literal YAML):

jobs:
  rust:
    steps:
      - checkout
      - install rust stable + wasm32-unknown-unknown target
      - install wasm-pack
      - cargo fmt --check                         # Format check
      - cargo clippy -- -D warnings               # Lint (warnings = errors)
      - cargo test                                 # Unit + integration tests
      - cargo tarpaulin --out xml                  # Coverage (gated >= 90%)
      - wasm-pack build --target web --release     # WASM compilation
      - check WASM size < 800KB gzipped            # Size gate
      - cargo deny check                           # License + advisory audit

  typescript:
    steps:
      - checkout
      - install node 20
      - install pnpm
      - pnpm install --frozen-lockfile
      - pnpm run lint                              # ESLint (warnings = errors)
      - pnpm run typecheck                         # tsc --noEmit
      - pnpm run format:check                      # Prettier --check
      - pnpm run test:coverage                     # Vitest (gated >= 90%)
      - pnpm run build                             # Production build
      - check bundle size < 1.5MB gzipped          # Size gate

  e2e:
    needs: [rust, typescript]
    steps:
      - checkout
      - build WASM + web (production)
      - install playwright browsers
      - pnpm run test:e2e                          # All E2E specs
      - upload test results + screenshots
```

### 20.2 Deploy Workflow (`.github/workflows/deploy.yml`)

Triggers: push to `main` only, after CI passes.

```yaml
# Conceptual steps:
jobs:
  deploy:
    steps:
      - checkout
      - full build (WASM + web)
      - wrangler pages deploy apps/web/dist
```

---

## 21. Git Workflow & Conventions

### 21.1 Branching Strategy

| Branch | Purpose | Protection |
|--------|---------|-----------|
| `main` | Production-ready code | Protected: require CI pass, no direct push |
| `feat/*` | Feature branches | PR to main |
| `fix/*` | Bug fix branches | PR to main |
| `chore/*` | Tooling, config, deps | PR to main |

### 21.2 Commit Message Format

**Standard:** [Conventional Commits 1.0.0](https://www.conventionalcommits.org/)

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

| Type | Usage |
|------|-------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no code change |
| `refactor` | Code change, no feature/fix |
| `perf` | Performance improvement |
| `test` | Adding/updating tests |
| `chore` | Build, CI, deps, tooling |

| Scope | Maps to |
|-------|---------|
| `wasm` | `crates/gerberview-wasm/` |
| `web` | `apps/web/` |
| `ci` | `.github/workflows/` |
| `deps` | Dependency updates |

**Examples:**
```
feat(wasm): implement circle aperture expansion
fix(web): correct zoom-to-cursor offset calculation
test(wasm): add boundary tests for zero-radius arc
chore(ci): add WASM binary size gate
```

### 21.3 Commitlint Configuration

```jsonc
// .commitlintrc.json
{
  "extends": ["@commitlint/config-conventional"],
  "rules": {
    "type-enum": [2, "always", [
      "feat", "fix", "docs", "style", "refactor", "perf", "test", "chore"
    ]],
    "scope-enum": [1, "always", ["wasm", "web", "ci", "deps"]],
    "subject-max-length": [2, "always", 72],
    "body-max-line-length": [2, "always", 100]
  }
}
```

---

## 22. Build & Development Workflow

### 22.1 Turborepo Pipeline

```jsonc
// turbo.json
{
  "$schema": "https://turbo.build/schema.json",
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**", "pkg/**"]
    },
    "lint": {},
    "typecheck": {},
    "test": {
      "dependsOn": ["build"]
    },
    "test:coverage": {
      "dependsOn": ["build"]
    },
    "test:e2e": {
      "dependsOn": ["build"]
    },
    "dev": {
      "cache": false,
      "persistent": true
    },
    "format:check": {}
  }
}
```

### 22.2 Root `package.json` Scripts

```jsonc
{
  "private": true,
  "scripts": {
    "build": "turbo run build",
    "dev": "turbo run dev",
    "lint": "turbo run lint",
    "typecheck": "turbo run typecheck",
    "format:check": "turbo run format:check",
    "test": "turbo run test",
    "test:coverage": "turbo run test:coverage",
    "test:e2e": "turbo run test:e2e",
    "wasm:build": "cd crates/gerberview-wasm && wasm-pack build --target web",
    "wasm:build:release": "cd crates/gerberview-wasm && wasm-pack build --target web --release",
    "prepare": "husky"
  }
}
```

### 22.3 Development Cycle

```
1. Edit Rust code
2. wasm-pack build (auto via vite-plugin-wasm-pack-watcher or manual)
3. Vite HMR reloads browser
4. Test: cargo test + pnpm run test
5. Commit: pre-commit hooks run lint-staged
6. Push: CI runs full pipeline
```

---

## 23. Security Considerations

| Concern | Mitigation |
|---------|-----------|
| ZIP bomb (compressed → huge decompressed) | Check uncompressed size before full extraction (JSZip provides this). Reject > 100MB. |
| Path traversal in ZIP filenames | Strip all directory components, use `basename` only. |
| Malicious Gerber content | `gerber_parser` is memory-safe Rust. No buffer overflows possible. |
| WASM memory exhaustion | Monitor `WebAssembly.Memory.buffer.byteLength`. Reject files that would produce >100M vertices (estimated via command count heuristic). |
| XSS via filename display | All user-provided strings rendered via `textContent`, never `innerHTML`. |
| Supply chain (npm) | `pnpm-lock.yaml` committed. `pnpm install --frozen-lockfile` in CI. |
| Supply chain (Cargo) | `Cargo.lock` committed. `cargo-deny` checks advisories. |
| CSP headers | Set via Cloudflare Pages `_headers` file: `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'` |

---

## 24. Logging & Observability

### 24.1 Rust/WASM Logging

```rust
// Use web_sys::console for all WASM logging.
use web_sys::console;

// Format: [GerberView] <component> <message> <metrics>
// Example: [GerberView] Parser: Parsed 4217 commands in 43ms
// Example: [GerberView] Geometry: Generated 18432 vertices for top_copper
// Example: [GerberView] Warning: Unsupported G74 arc mode, skipping

// Levels:
// console::log_1     → info / metrics
// console::warn_1    → recoverable issues
// console::error_1   → failures
```

### 24.2 TypeScript Logging

```typescript
// Use console.group / console.time for structured output.
// No console.log in production (ESLint enforced).
// Allowed: console.warn, console.error, console.group, console.groupEnd,
//          console.time, console.timeEnd

// Format:
// [GerberView] Upload: Extracted 7 files from board.zip (23ms)
// [GerberView] Render: Uploaded 6 VBOs, 142KB total
// [GerberView] Error: WebGL context lost
```

### 24.3 Performance Marks

```typescript
performance.mark("parse-start");
// ... parse all layers ...
performance.mark("parse-end");
performance.measure("parse-total", "parse-start", "parse-end");

performance.mark("render-start");
// ... upload VBOs + first draw ...
performance.mark("render-end");
performance.measure("render-total", "render-start", "render-end");
```

These marks are available in browser DevTools Performance panel and in Playwright E2E tests.

---

## 25. Dependency Management

### 25.1 Rust

| Policy | Rule |
|--------|------|
| Lock file | `Cargo.lock` committed to repo |
| Version specifiers | Use `"major.minor"` (e.g., `"0.4"`) for flexibility within minor |
| Auditing | `cargo-deny` in CI: deny known vulnerabilities, deny copyleft licenses |
| Updates | Manual via `cargo update`. Review changelogs before merging. |
| New dependencies | Must be justified in PR description. Must pass `cargo-deny`. |

### 25.2 TypeScript

| Policy | Rule |
|--------|------|
| Package manager | pnpm (strict, deterministic, fast) |
| Lock file | `pnpm-lock.yaml` committed |
| Install mode | `--frozen-lockfile` in CI |
| Version specifiers | Exact versions in `package.json` (no `^` or `~`) |
| Updates | Dependabot or Renovate PR, reviewed before merge |
| New dependencies | Must be justified. Prefer zero-dependency solutions. |

---

## 26. Glossary

| Term | Definition |
|------|-----------|
| **Aperture** | A tool shape used for drawing and flashing in Gerber files (circle, rectangle, obround, polygon, or macro-defined). |
| **D-code** | A command in Gerber that selects (D10+), draws (D01), moves (D02), or flashes (D03) with the current aperture. |
| **Excellon** | A file format for CNC drill machines, used to specify hole locations and sizes in PCBs. |
| **Flash (D03)** | Place the current aperture shape at a coordinate without drawing a line. |
| **Gerber RS-274X** | The extended Gerber format, the industry standard for PCB layer data exchange. Supersedes RS-274D. |
| **MSRV** | Minimum Supported Rust Version — the oldest Rust compiler the crate promises to build on. |
| **Polarity** | Whether drawn shapes add (dark/LPD) or subtract (clear/LPC) from the layer image. |
| **Region (G36/G37)** | A filled polygon defined by a boundary path in Gerber format. |
| **Step-Repeat (SR)** | A Gerber block that repeats contained geometry in a grid pattern. |
| **Tessellation** | Converting a curved shape (arc, circle) into a series of straight-line segments for rendering. |
| **VBO** | Vertex Buffer Object — a GPU buffer holding vertex data for WebGL rendering. |
| **WASM** | WebAssembly — a binary instruction format for stack-based virtual machines, used as a compilation target for Rust. |

---

> **End of Specification**  
> **Next step:** Phase 0 — Project scaffolding per this spec.
