# GerberView — Task Breakdown

> **Document ID:** GVTASK-001  
> **Version:** 1.0.0  
> **Date:** 2026-02-21  
> **Upstream:** [Spec](./gerber-viewer-spec.md), [Architecture](./gerber-viewer-architecture.md), [Feasibility](./gerber-viewer-feasibility.md), [Brief](./gerber-viewer-agent-brief.md)

---

## Conventions

- **Effort**: S (< 1h), M (1-3h), L (3-6h), XL (6-12h)
- **Dependencies**: Listed by task ID. A task MUST NOT start until all listed dependencies are complete.
- **Commit convention**: `type(scope): description` per spec Section 21.2
- All tasks produce code that passes `cargo fmt`, `cargo clippy -- -D warnings`, `eslint`, `prettier`, `tsc --noEmit` as applicable.

### Git Branching Workflow

Every task (except T-00 which is the initial commit on `main`) follows this lifecycle:

**Start of task:**
```bash
git checkout main && git pull origin main
git checkout -b <type>/T-XX-<short-name>
```

**During task:** Commit incrementally using Conventional Commits.

**End of task:**
```bash
git push -u origin HEAD
gh pr create --title "<type>(scope): T-XX description" --body "Closes T-XX"
gh pr merge --squash --delete-branch
git checkout main && git pull origin main
```

Branch type prefixes follow spec Section 21.1: `feat/`, `fix/`, `chore/`, or `test/`.  
Each task table includes a **Branch** field with the exact branch name to use.

**Every task's Definition of Done implicitly includes:** branch pushed, PR created, PR merged to `main`, local `main` checked out and synced with remote. The next task MUST start from an up-to-date `main`.

---

## Phase 0 — Project Foundation

### T-00: Initialize Git Repository + README

| Field | Detail |
|-------|--------|
| **Effort** | S |
| **Branch** | `main` (initial commit — no feature branch) |
| **Background** | Every task depends on a repository existing. The README serves as the project's public face and is required for the portfolio use case (brief Section 1). |
| **Description** | Create a GitHub repository. Add a README with project title, one-liner description, tech stack badges, and "Work in Progress" notice. Add MIT LICENSE and a comprehensive `.gitignore` for Rust, Node, WASM, and IDE files. |
| **Scope** | Root-level files only. No source code. |
| **Files to Create** | `README.md`, `LICENSE`, `.gitignore` |
| **Dependencies** | None |
| **Definition of Done** | Public GitHub repo exists. `README.md` contains project name, one-liner, tech stack list, MIT license badge. `.gitignore` covers `target/`, `node_modules/`, `dist/`, `pkg/`, `.wasm`, `.env`, IDE folders. LICENSE is MIT with correct year/author. |
| **Test Criteria** | `git status` shows clean working tree after initial commit. |

---

### T-01: Rust Crate Scaffold + WASM Build Verification

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `chore/T-01-rust-scaffold` |
| **Background** | The single biggest risk is whether `gerber_parser` compiles to `wasm32-unknown-unknown` (feasibility Section 1). This task eliminates that risk before any other Rust code is written. |
| **Description** | Create the virtual Cargo workspace manifest and the `gerberview-wasm` crate. Add all Rust dependencies from spec Section 3.1 (gerber_parser 0.4, wasm-bindgen, web-sys, js-sys, serde, serde-wasm-bindgen, earclip, console_error_panic_hook). Add `wasm-bindgen-test` as dev-dependency. Set release profile per feasibility (`opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = true`). Add `rustfmt.toml` and `clippy.toml` per spec Section 11.2-11.3. Create minimal `lib.rs` with crate-level deny attributes (spec Section 11.1) and a single `#[wasm_bindgen] pub fn ping() -> u32 { 42 }` export. Verify `wasm-pack build --target web` succeeds. |
| **Scope** | `crates/gerberview-wasm/` directory only. No geometry, no parsing, no web app. |
| **Files to Create** | `Cargo.toml` (workspace root), `crates/gerberview-wasm/Cargo.toml`, `crates/gerberview-wasm/src/lib.rs`, `rustfmt.toml`, `clippy.toml`, `deny.toml` |
| **Dependencies** | T-00 |
| **Definition of Done** | `cargo build --target wasm32-unknown-unknown` succeeds. `wasm-pack build --target web` produces `pkg/` output. `cargo fmt --check` passes. `cargo clippy -- -D warnings` passes. `cargo deny check` passes (licenses + advisories). |
| **Test Criteria** | `cargo test` passes (trivial test for `ping()`). `wasm-pack test --headless --chrome` passes with `wasm-bindgen-test`. |

---

### T-02: Web Application Scaffold (Vite + TypeScript + Tailwind)

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `chore/T-02-web-scaffold` |
| **Background** | The frontend needs a build system that integrates with wasm-pack. Vite + `vite-plugin-wasm-pack` is the validated approach (feasibility Section 6). |
| **Description** | Initialize `apps/web/` with Vite, TypeScript, and Tailwind CSS. Configure `vite.config.ts` with `vite-plugin-wasm-pack` pointing to `../../crates/gerberview-wasm` and `vite-plugin-top-level-await`. Set up `tsconfig.json` per spec Section 12.1 (strict mode, all extra flags). Create `tailwind.config.ts` per spec Section 14.1. Add `index.html` with dark background, canvas element, and the structural HTML from the wireframe (brief Section 17). Create `src/main.ts` that imports the WASM module and calls `ping()`, logging the result to console. Verify `pnpm run dev` serves a page that loads WASM. |
| **Scope** | `apps/web/` directory. The page displays "GerberView" header and a dark canvas area. No functionality beyond WASM load verification. |
| **Files to Create** | `apps/web/package.json`, `apps/web/tsconfig.json`, `apps/web/vite.config.ts`, `apps/web/tailwind.config.ts`, `apps/web/postcss.config.js`, `apps/web/index.html`, `apps/web/src/main.ts`, `apps/web/src/styles/main.css` |
| **Dependencies** | T-01 |
| **Definition of Done** | `pnpm run dev` starts Vite dev server. Browser loads page, WASM initializes, console shows `42` from `ping()`. `tsc --noEmit` passes. Tailwind compiles. Page has dark background (`#1a1a1a`). |
| **Test Criteria** | Manual: page loads, console output correct. Automated: `pnpm run build` produces `dist/` with all assets. |

---

### T-03: Monorepo Tooling + Code Quality Gates

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `chore/T-03-monorepo-tooling` |
| **Background** | The spec mandates Turborepo for monorepo orchestration, husky+lint-staged for pre-commit hooks, commitlint for Conventional Commits, and shared ESLint configuration (spec Sections 15, 21, 22). All warnings are errors. |
| **Description** | Set up root `package.json` with pnpm workspaces. Create `pnpm-workspace.yaml`. Configure `turbo.json` per spec Section 22.1. Create `packages/eslint-config/` with the shared ESLint flat config from spec Section 12.2. Set up Prettier config (`.prettierrc`) per spec Section 12.3. Install and configure husky (pre-commit + commit-msg hooks). Configure lint-staged per spec Section 15.3. Set up commitlint per spec Section 21.3. Configure Vitest in `apps/web/package.json`. Configure Playwright in `apps/web/e2e/playwright.config.ts`. Add all npm scripts per spec Section 22.2. |
| **Scope** | Root config files, `packages/eslint-config/`, hook scripts. No application code changes. |
| **Files to Create** | `pnpm-workspace.yaml`, `turbo.json`, `.commitlintrc.json`, `.prettierrc`, `.husky/pre-commit`, `.husky/commit-msg`, `packages/eslint-config/package.json`, `packages/eslint-config/index.js`, `apps/web/e2e/playwright.config.ts` |
| **Files to Change** | `package.json` (root — add workspaces, scripts, devDeps), `apps/web/package.json` (add ESLint, Vitest, Playwright deps + scripts) |
| **Dependencies** | T-02 |
| **Definition of Done** | `turbo run lint` executes ESLint on `apps/web/`. `turbo run build` builds WASM then web. Pre-commit hook runs lint-staged on staged `.ts` and `.rs` files. `git commit` with non-Conventional message is rejected by commitlint. `pnpm run format:check` passes. `pnpm run typecheck` passes. |
| **Test Criteria** | Commit a file with a lint error → pre-commit hook blocks. Commit with `bad message` → commit-msg hook blocks. Commit with `chore(web): add tooling` → succeeds. `turbo run lint typecheck format:check` all pass. |

---

### T-04: CI Pipeline (GitHub Actions)

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `chore/T-04-ci-pipeline` |
| **Background** | CI is the enforcement mechanism for all quality gates (spec Section 20). Without CI, lint/test/coverage rules are advisory only. |
| **Description** | Create `.github/workflows/ci.yml` with three jobs per spec Section 20.1: `rust` (fmt, clippy, test, wasm-pack build, cargo-deny), `typescript` (lint, typecheck, format:check, test, build), and `e2e` (placeholder — enabled when E2E tests exist). Create `.github/workflows/deploy.yml` (placeholder — builds but does not deploy until Cloudflare is configured). Triggers: CI on every push and PR; deploy on push to `main` only. |
| **Scope** | `.github/workflows/` only. |
| **Files to Create** | `.github/workflows/ci.yml`, `.github/workflows/deploy.yml` |
| **Dependencies** | T-03 |
| **Definition of Done** | Push to a feature branch triggers CI. All three jobs pass. Badge in README shows CI status. Deploy workflow exists but is no-op until Cloudflare config. |
| **Test Criteria** | Push a commit → GitHub Actions runs → all jobs green. Push a commit with a `cargo fmt` violation → `rust` job fails. |

---

### T-05: Test Fixture Files

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `chore/T-05-test-fixtures` |
| **Background** | Deterministic testing requires committed fixture files, not runtime downloads (spec Section 4.1). The spec identifies KiCad sample, Arduino Uno, and Eagle boards as fixtures. E2E tests need pre-built ZIP files. |
| **Description** | Download open-source Gerber files: KiCad demo project, Arduino Uno (from arduino.cc), and a SparkFun Eagle board. Place them in `crates/gerberview-wasm/tests/fixtures/` organized by source. Create hand-crafted minimal Gerber files for unit testing (simple rectangle, single circle flash, single arc, simple region, drill file). Create E2E fixture ZIPs: `kicad-sample.zip`, `eagle-sample.zip`, `empty.zip`, `invalid.zip` (non-ZIP binary), `no-gerber.zip` (ZIP with only a README). Place in `apps/web/e2e/fixtures/`. |
| **Scope** | Test data only. No code changes. |
| **Files to Create** | `crates/gerberview-wasm/tests/fixtures/kicad-sample/*.gbr`, `crates/gerberview-wasm/tests/fixtures/arduino-uno/*`, `crates/gerberview-wasm/tests/fixtures/eagle-sample/*`, `crates/gerberview-wasm/tests/fixtures/minimal/*.gbr` (hand-crafted), `apps/web/e2e/fixtures/*.zip` |
| **Dependencies** | T-01, T-03 |
| **Definition of Done** | `crates/gerberview-wasm/tests/fixtures/` contains at least 3 real-world board directories and 5+ minimal hand-crafted Gerber files. `apps/web/e2e/fixtures/` contains 5 ZIP files (2 valid boards, 1 empty, 1 invalid, 1 no-gerber). All committed to repo. |
| **Test Criteria** | `ls crates/gerberview-wasm/tests/fixtures/kicad-sample/` shows at least 7 Gerber files (copper, mask, silk, outline, drill). |

---

## Phase 1 — Core Types & Infrastructure

### T-06: Rust Core Types + Error Types + GeometryBuilder

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-06-rust-core-types` |
| **Background** | Every Rust module depends on shared types (spec Section 7.1-7.2). The `GeometryBuilder` is the central accumulator used by all geometry sub-modules (architecture Section 10.2). Error types must be defined before any fallible function is written (spec Section 10.1). |
| **Description** | Create `crates/gerberview-wasm/src/geometry/types.rs` with `Point`, `BoundingBox`, `LayerGeometry`, `LayerMeta`, `GerberState`, `InterpolationMode`, `Polarity` per spec Section 7.1-7.2. Implement `GeometryBuilder` with `push_vertex`, `push_triangle`, `push_quad`, `push_ngon`, `warn`, `build` methods per architecture Section 10.2. Create `crates/gerberview-wasm/src/error.rs` with `GeometryError` enum (variants: `InvalidAperture`, `DegenerateGeometry`, `UnsupportedFeature`, `ArcError`, `RegionError`, `MacroError`, `ParseError`). All variants carry a descriptive `String`. Implement `Display` via `thiserror`. Create `crates/gerberview-wasm/src/excellon/types.rs` with `DrillHole`, `ToolDefinition`, `ExcellonResult`, `ExcellonUnits`. Create `geometry/mod.rs` and `excellon/mod.rs` as re-export hubs. |
| **Scope** | Type definitions and `GeometryBuilder` only. No parsing, no geometry algorithms. |
| **Files to Create** | `crates/gerberview-wasm/src/geometry/types.rs`, `crates/gerberview-wasm/src/geometry/mod.rs`, `crates/gerberview-wasm/src/error.rs`, `crates/gerberview-wasm/src/excellon/types.rs`, `crates/gerberview-wasm/src/excellon/mod.rs` |
| **Files to Change** | `crates/gerberview-wasm/src/lib.rs` (add module declarations) |
| **Dependencies** | T-01 |
| **Definition of Done** | `cargo build` passes. `cargo clippy -- -D warnings` passes. `cargo doc` generates docs for all public types. All pub items have `///` doc comments. `#![deny(missing_docs)]` satisfied. |
| **Test Criteria** | Unit tests for `GeometryBuilder`: push 3 vertices → positions has 6 floats. Push triangle → indices has 3 entries. `push_ngon(0,0,1,4)` → 4 vertices on unit circle. `build()` returns correct `vertex_count` and `bounds`. `BoundingBox` updates correctly as vertices are added. ~10 tests. |

---

### T-07: TypeScript Types + Constants + Signals + Store

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-07-ts-types-store` |
| **Background** | The reactive store is the backbone of the TS application (architecture Section 5). Types must exist before any module can be written. The `Signal<T>` primitive is ~40 LOC but everything depends on it. |
| **Description** | Create `apps/web/src/types.ts` with all TS types from spec Section 7.3 plus worker message types from architecture Section 7.1-7.2 plus missing types (`Point`, `LoadingProgress`). Create `apps/web/src/constants.ts` with the layer color map (spec Section 7.4), z-order table (architecture Section 6.4), `ViewerConfig` defaults. Create `apps/web/src/core/signal.ts` implementing `Signal<T>`, `Computed<T>`, and `ReadonlySignal<T>` per architecture Section 5.1-5.2. Create `apps/web/src/core/store.ts` implementing `AppStore` per architecture Section 5.3 with `createAppStore()` factory function. |
| **Scope** | Core data layer. No DOM, no WebGL, no Worker. |
| **Files to Create** | `apps/web/src/types.ts`, `apps/web/src/constants.ts`, `apps/web/src/core/signal.ts`, `apps/web/src/core/store.ts` |
| **Dependencies** | T-03 |
| **Definition of Done** | `tsc --noEmit` passes. `eslint` passes. All types exported. `createAppStore()` returns a fully initialized store with all signals and computed values. |
| **Test Criteria** | `__tests__/signal.test.ts`: Signal set value → subscriber called. Unsubscribe → no longer called. Computed recomputes on dependency change. `__tests__/store.test.ts`: `createAppStore()` → all signals have correct defaults. Toggle layer visibility → `visibleLayers` computed updates. ~15 tests. |

---

### T-08: Layer Identification Module

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `feat/T-08-layer-identify` |
| **Background** | Layer identification runs before WASM parsing — it determines which parser to call and what color to assign (architecture Section 3, engine layer). Pattern tables are defined in brief Section 16 and spec Section 5.2. |
| **Description** | Create `apps/web/src/engine/layer-identify.ts` implementing `identifyLayer(fileName: string): IdentifiedFile` (without `content` — that's added by zip-handler). Support KiCad, Eagle, Altium, EasyEDA, and Protel filename patterns. Case-insensitive matching. Content-based fallback stub (checks for `%FSLAX` / `M48` magic bytes — full implementation deferred to zip-handler which has file content). |
| **Scope** | Pure function, no side effects, no dependencies beyond `types.ts` and `constants.ts`. |
| **Files to Create** | `apps/web/src/engine/layer-identify.ts` |
| **Dependencies** | T-07 |
| **Definition of Done** | `eslint` + `tsc --noEmit` pass. Function handles all patterns from spec Section 5.2 / brief Section 16. Unknown files return `{ layerType: "unknown", fileType: "unknown" }`. |
| **Test Criteria** | `__tests__/layer-identify.test.ts`: Spec tests UT-TS-001 through UT-TS-007. Additionally: Eagle `.cmp`/`.sol`, Altium `.GTS`/`.GBS`, EasyEDA `Gerber_TopLayer.GTL`, drill `.drl`/`.xln`/`.DRL`. Edge cases: no extension, double extension `.gbr.bak`, mixed case `.GtL`. ~20 tests. |

---

### T-09: ZIP Handler Module

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `feat/T-09-zip-handler` |
| **Background** | The ZIP handler is the entry point for user files (architecture Section 11.1 steps 1-5). It validates, extracts, and identifies files before handing them to the worker. Boundary conditions are exhaustive (spec Section 9.1). |
| **Description** | Create `apps/web/src/engine/zip-handler.ts` implementing `extractAndIdentify(file: File): Promise<IdentifiedFile[]>`. Uses JSZip for extraction. Validates: is it a ZIP? Is it empty? Are there any Gerber/Excellon files? Is uncompressed size < 100MB? Strips path traversal from filenames. Calls `identifyLayer()` for each file. Filters out unknown files. Returns array of `IdentifiedFile` with `content: Uint8Array` populated. Content-based fallback: if filename matching returns unknown, inspect first 100 bytes for `%FSLAX` (Gerber) or `M48` (Excellon). |
| **Scope** | File handling only. No WASM, no rendering, no state mutation. |
| **Files to Create** | `apps/web/src/engine/zip-handler.ts` |
| **Dependencies** | T-08 |
| **Definition of Done** | `eslint` + `tsc --noEmit` pass. All 10 ZIP boundary conditions from spec Section 9.1 are handled. Errors thrown are typed `AppError`. |
| **Test Criteria** | `__tests__/zip-handler.test.ts`: Spec tests UT-TS-012 through UT-TS-014. Additional: nested directories flattened, path traversal stripped, unicode filenames handled, content-based fallback identifies Gerber from unknown extension. Requires mock JSZip or fixture files. ~12 tests (BC-ZIP-001 through BC-ZIP-010). |

---

## Phase 2 — Parsing Pipeline

### T-10: WASM Bridge (lib.rs) + parse_gerber Export

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `feat/T-10-wasm-bridge` |
| **Background** | The WASM bridge is the boundary between Rust and JS (architecture Section 8). This task connects `gerber_parser::parse()` to a `#[wasm_bindgen]` export and verifies the full parse pipeline works. |
| **Description** | Replace the `ping()` stub in `lib.rs` with the real exports: `parse_gerber(data: &[u8]) -> Result<JsValue, JsValue>` and `parse_excellon(data: &[u8]) -> Result<JsValue, JsValue>` (excellon body stubbed). Add `get_positions() -> Vec<f32>` and `get_indices() -> Vec<u32>`. Implement `parse_gerber`: wrap input in `BufReader::new(Cursor::new(data))`, call `gerber_parser::parse()`, store result in `thread_local! { LAST_GEOMETRY }`, return `LayerMeta` via `serde_wasm_bindgen::to_value()`. Geometry conversion stubbed to produce empty geometry (no vertices) — actual conversion is T-18. Add `#[wasm_bindgen(start)]` init function with `console_error_panic_hook`. |
| **Scope** | `lib.rs` only. Parsing works but produces zero geometry (converter not yet implemented). |
| **Files to Change** | `crates/gerberview-wasm/src/lib.rs` |
| **Dependencies** | T-06 |
| **Definition of Done** | `wasm-pack build --target web` succeeds. `parse_gerber` accepts bytes, calls `gerber_parser::parse()`, returns `LayerMeta` as JsValue with `vertex_count: 0`. `get_positions()` / `get_indices()` return empty Vecs. No panics on valid or invalid input. |
| **Test Criteria** | Rust test: parse a minimal fixture Gerber file → returns Ok with command_count > 0, vertex_count = 0. Parse empty bytes → returns Err. Parse garbage bytes → returns Err, no panic. `wasm-bindgen-test`: call from JS context, verify JsValue round-trip. ~5 tests. |

---

### T-11: Excellon Drill Parser

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-11-excellon-parser` |
| **Background** | No Rust crate exists for Excellon parsing (feasibility Section 4). The format is simple (~200-400 LOC). This is the only parser we write from scratch. |
| **Description** | Implement `crates/gerberview-wasm/src/excellon/parser.rs` with `pub fn parse_excellon(data: &[u8]) -> Result<ExcellonResult, GeometryError>`. Parse: `M48` header, tool definitions (`T<n>C<diameter>`), unit declarations (`METRIC`/`INCH`), zero suppression (`LZ`/`TZ`), coordinate format, tool selection in body, hole coordinates (`X<n>Y<n>`), `M30` end-of-file. Convert parsed holes to geometry (circle N-gon flash per hole) and store as `LayerGeometry`. Wire into `lib.rs` `parse_excellon` export. |
| **Scope** | `excellon/parser.rs` and `lib.rs` update. Drill holes only — no routing (G00/G01-G03 in body are ignored). |
| **Files to Create** | `crates/gerberview-wasm/src/excellon/parser.rs` |
| **Files to Change** | `crates/gerberview-wasm/src/excellon/mod.rs`, `crates/gerberview-wasm/src/lib.rs` |
| **Dependencies** | T-06, T-10 |
| **Definition of Done** | `cargo test` passes. Parse a real drill file from fixtures → correct tool count, hole count, positions. `clippy` + `fmt` pass. All 8 Excellon boundary conditions (spec Section 9.3) handled. |
| **Test Criteria** | Spec tests UT-EXC-001 through UT-EXC-006 plus boundary conditions BC-EXC-001 through BC-EXC-008. ~14 tests total. |

---

### T-12: Web Worker + WorkerClient

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-12-web-worker` |
| **Background** | Parsing runs off the main thread (architecture Section 4, Decision AD-001). The Worker loads WASM, receives file bytes via Transferable, parses, and returns buffers. The WorkerClient provides a typed async API. |
| **Description** | Create `apps/web/src/engine/parse-worker.ts` (Web Worker entry point): import and init WASM module, listen for `ParseRequestMessage`, call `parse_gerber`/`parse_excellon` per file, post `LayerResultMessage` or `LayerErrorMessage`, post `ParseCompleteMessage` when done. Transfer `positions.buffer` and `indices.buffer`. Create `apps/web/src/engine/worker-client.ts` with `WorkerClient` class: `waitForReady()`, `parseFiles()` async generator, `cancel()`, `dispose()` per architecture Section 17.1. Handle request ID matching for cancellation (architecture Section 7.4). |
| **Scope** | Worker communication only. Does not touch store, scene, or renderer. |
| **Files to Create** | `apps/web/src/engine/parse-worker.ts`, `apps/web/src/engine/worker-client.ts` |
| **Dependencies** | T-07, T-10 |
| **Definition of Done** | `tsc --noEmit` passes. `WorkerClient.waitForReady()` resolves after WASM init. `parseFiles()` yields `LayerResultMessage` for each valid file and `LayerErrorMessage` for invalid files. Cancellation via `cancel()` causes stale results to be dropped. Buffers are Transferred (not copied). |
| **Test Criteria** | Integration test (manual or Vitest with Worker polyfill): send a fixture Gerber file → receive layer-result with meta. Send invalid file → receive layer-error. Call cancel mid-parse → stale results ignored. ~5 tests. |

---

## Phase 3 — Geometry Engine

### T-13: Aperture Expansion (Circle, Rect, Obround, Polygon)

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-13-aperture-expansion` |
| **Background** | Aperture flashing (D03) is the most common Gerber operation — every pad, via, and test point is a flash. Four standard shapes must be supported (spec FR-301, brief Section 9.1). |
| **Description** | Create `crates/gerberview-wasm/src/geometry/aperture.rs` with `pub fn flash_aperture(builder: &mut GeometryBuilder, aperture: &Aperture, position: Point) -> Result<(), GeometryError>`. Implement: Circle → N-gon (32 segments), Rectangle → 4 vertices + 2 triangles, Obround → rectangle body + 2 semicircle endcaps, Polygon → regular N-gon with rotation. Handle boundary conditions: zero diameter, negative dimensions. |
| **Scope** | `aperture.rs` only. Pure geometry — no parsing, no state machine. |
| **Files to Create** | `crates/gerberview-wasm/src/geometry/aperture.rs` |
| **Files to Change** | `crates/gerberview-wasm/src/geometry/mod.rs` (add pub use) |
| **Dependencies** | T-06 |
| **Definition of Done** | `cargo test` passes. `clippy` passes. All four aperture types produce correct vertices. Boundary conditions BC-GBR-007 and BC-GBR-008 handled (zero diameter → warning, negative dims → absolute value). |
| **Test Criteria** | Spec tests UT-APR-001 through UT-APR-009. ~9 tests. |

---

### T-14: Stroke Widening (D01 Linear Draw)

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `feat/T-14-stroke-widening` |
| **Background** | D01 linear interpolation creates thick lines (traces, board edges). The stroke must be expanded into quads with optional semicircle endcaps (brief Section 9.2). |
| **Description** | Create `crates/gerberview-wasm/src/geometry/stroke.rs` with `pub fn draw_linear(builder: &mut GeometryBuilder, from: Point, to: Point, aperture: &Aperture) -> Result<(), GeometryError>`. Calculate direction vector, perpendicular offset, 4 corner vertices, 2 triangles for body. Add semicircle endcaps for circular apertures. Handle zero-length line (degenerate → circle flash or skip with warning). |
| **Scope** | `stroke.rs` only. |
| **Files to Create** | `crates/gerberview-wasm/src/geometry/stroke.rs` |
| **Files to Change** | `crates/gerberview-wasm/src/geometry/mod.rs` |
| **Dependencies** | T-06, T-13 (reuses `push_ngon` for endcaps) |
| **Definition of Done** | `cargo test` + `clippy` pass. Horizontal, vertical, and diagonal lines produce correct quads. Endcaps added for circular apertures. |
| **Test Criteria** | Spec tests UT-STR-001 through UT-STR-006. ~6 tests. |

---

### T-15: Arc Tessellation (G02/G03)

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-15-arc-tessellation` |
| **Background** | Arcs appear in board outlines, rounded traces, and pad connections. The math is well-defined but edge cases are tricky — full circles, near-zero arcs, radius mismatches (architecture Section 18.4, brief Section 9.3). |
| **Description** | Create `crates/gerberview-wasm/src/geometry/arc.rs` with `pub fn draw_arc(builder: &mut GeometryBuilder, from: Point, to: Point, center_offset: Point, direction: ArcDirection, aperture: &Aperture) -> Result<(), GeometryError>`. Implement: center computation, radius validation, start/end angle calculation, sweep direction (CW/CCW) per architecture Section 18.4, adaptive tessellation (`N = max(16, ...)`), stroke-widening per segment (reuse `stroke.rs`). Handle: G75 multi-quadrant mode only. Log warning for G74. Zero-radius → skip. Full circle (start==end, I,J!=0) → 360° sweep. Radius mismatch → warn and use average. |
| **Scope** | `arc.rs` only. |
| **Files to Create** | `crates/gerberview-wasm/src/geometry/arc.rs` |
| **Files to Change** | `crates/gerberview-wasm/src/geometry/mod.rs` |
| **Dependencies** | T-14 (reuses `draw_linear` for segment widening) |
| **Definition of Done** | `cargo test` + `clippy` pass. All arc boundary conditions from spec Section 9.2 (BC-GBR-012 through BC-GBR-015) handled. |
| **Test Criteria** | Spec tests UT-ARC-001 through UT-ARC-007. ~7 tests. |

---

### T-16: Region Fill (G36/G37)

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `feat/T-16-region-fill` |
| **Background** | Regions are filled polygons (copper pours, ground planes). Triangulation is delegated to the `earclip` crate (feasibility Section 3.1). Boundary can include arcs that must be tessellated to line segments first. |
| **Description** | Create `crates/gerberview-wasm/src/geometry/region.rs` with `pub fn fill_region(builder: &mut GeometryBuilder, boundary: &[Point]) -> Result<(), GeometryError>`. Convert boundary points to `earclip` input format. Call `earclip::triangulate`. Push resulting triangles to builder. Handle: < 3 points → skip + warn. Unclosed polygon → auto-close. Self-intersecting → earclip handles. |
| **Scope** | `region.rs` only. |
| **Files to Create** | `crates/gerberview-wasm/src/geometry/region.rs` |
| **Files to Change** | `crates/gerberview-wasm/src/geometry/mod.rs` |
| **Dependencies** | T-06 |
| **Definition of Done** | `cargo test` + `clippy` pass. Square → 2 triangles. L-shape → correct triangulation. Degenerate inputs handled per spec Section 9.2 (BC-GBR-016 through BC-GBR-018). |
| **Test Criteria** | Spec tests UT-REG-001 through UT-REG-008. ~8 tests. |

---

### T-17: Polarity + Step-Repeat + Aperture Macros

| Field | Detail |
|-------|--------|
| **Effort** | XL |
| **Branch** | `feat/T-17-polarity-sr-macros` |
| **Background** | These three features are less frequently used but necessary for real-world boards. Polarity is MVP Option B (clear = background color). Step-repeat is vertex duplication. Aperture macros are the most complex Gerber feature. |
| **Description** | Create `crates/gerberview-wasm/src/geometry/polarity.rs`: track polarity state, mark clear-polarity geometry with background color flag. Create `crates/gerberview-wasm/src/geometry/step_repeat.rs`: duplicate vertex ranges with X/Y offsets for each grid position. Create `crates/gerberview-wasm/src/geometry/macro_eval.rs`: evaluate aperture macro primitives (Circle/1, VectorLine/20, CenterLine/21, Outline/4, Polygon/5) with exposure flags and arithmetic expression evaluation. Each primitive produces vertices via `GeometryBuilder`. |
| **Scope** | Three files. Macro evaluator is the bulk of effort (~200-300 LOC). |
| **Files to Create** | `crates/gerberview-wasm/src/geometry/polarity.rs`, `crates/gerberview-wasm/src/geometry/step_repeat.rs`, `crates/gerberview-wasm/src/geometry/macro_eval.rs` |
| **Files to Change** | `crates/gerberview-wasm/src/geometry/mod.rs` |
| **Dependencies** | T-13, T-14 |
| **Definition of Done** | `cargo test` + `clippy` pass. Polarity: dark → normal, clear → background flag. Step-repeat: 2x3 grid → 6 copies with correct offsets. Macros: all 5 primitive types produce geometry. Arithmetic expressions evaluated. Boundary conditions BC-GBR-019, BC-GBR-020, BC-GBR-024, BC-GBR-025 handled. |
| **Test Criteria** | Spec tests UT-POL-001 through UT-POL-003, UT-SR-001 through UT-SR-003, UT-MAC-001 through UT-MAC-005. ~11 tests. |

---

### T-18: Geometry Converter Orchestrator

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-18-geometry-orchestrator` |
| **Background** | The orchestrator walks the `GerberDoc` command list, maintains `GerberState`, and dispatches to the correct sub-module for each command (architecture Section 10.1). This is where the state machine lives. |
| **Description** | Implement `pub fn convert(doc: &GerberDoc) -> Result<LayerGeometry, GeometryError>` in `crates/gerberview-wasm/src/geometry/mod.rs`. Create `GerberState` instance. Iterate `doc.commands()`. For each `Command` variant: update state (aperture selection, interpolation mode, polarity, region mode), dispatch to `aperture::flash_aperture` (D03), `stroke::draw_linear` (D01+G01), `arc::draw_arc` (D01+G02/G03), collect region points (G36→G37) then `region::fill_region`, handle step-repeat blocks, handle aperture macro flashes. Convert coordinates using `doc.format_specification` and `doc.units`. Wire `convert()` into `lib.rs` `parse_gerber` so it produces real geometry. |
| **Scope** | `geometry/mod.rs` implementation + `lib.rs` wiring. |
| **Files to Change** | `crates/gerberview-wasm/src/geometry/mod.rs`, `crates/gerberview-wasm/src/lib.rs` |
| **Dependencies** | T-10, T-13, T-14, T-15, T-16, T-17 |
| **Definition of Done** | Parse a real KiCad copper layer fixture → `LayerGeometry` with `vertex_count > 0`, valid `bounds`, `positions.len() == vertex_count * 2`, all indices valid. No panics on any fixture file. |
| **Test Criteria** | IT-001 through IT-003 (spec Section 16.4): parse real files → non-empty geometry with reasonable bounds. Parse malformed file → partial result + error (IT-007). ~5 integration tests. |

---

### T-19: Rust Integration Tests + Benchmarks

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `test/T-19-rust-integration-tests` |
| **Background** | Integration tests validate the full parse→geometry pipeline against real boards. Benchmarks establish performance baselines (spec Section 16.4, 16.7). |
| **Description** | Create `crates/gerberview-wasm/tests/parse_test.rs`, `geometry_test.rs`, `excellon_test.rs` per spec Section 16.4. Test: KiCad board → all layers produce geometry. Eagle board → compatible output. Arduino Uno → bounds match expected. Malformed file → no panic. Create `crates/gerberview-wasm/benches/parse_bench.rs` using criterion: benchmark parse time and geometry conversion time for KiCad sample. |
| **Scope** | `tests/` and `benches/` only. |
| **Files to Create** | `crates/gerberview-wasm/tests/parse_test.rs`, `crates/gerberview-wasm/tests/geometry_test.rs`, `crates/gerberview-wasm/tests/excellon_test.rs`, `crates/gerberview-wasm/benches/parse_bench.rs` |
| **Files to Change** | `crates/gerberview-wasm/Cargo.toml` (add `criterion` dev-dependency + `[[bench]]` entry) |
| **Dependencies** | T-18, T-11, T-05 |
| **Definition of Done** | `cargo test` passes all integration tests. `cargo bench` produces timing results. Coverage >= 90% lines (measured by `cargo tarpaulin`). IT-001 through IT-007 pass. PERF-001 (< 500ms parse) and PERF-002 (< 1000ms geometry) pass for KiCad sample. |
| **Test Criteria** | All 7 integration tests from spec Section 16.4. 2 benchmarks from spec Section 16.7 (PERF-001, PERF-002). |

---

## Phase 4 — WebGL Rendering

### T-20: WebGL Setup + Shader Programs

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `feat/T-20-webgl-shaders` |
| **Background** | WebGL 1.0 is the rendering target (spec FR-400). Shaders are trivial (flat color + view matrix) but the setup boilerplate (context, extension, compile, link) must be robust. |
| **Description** | Create `apps/web/src/render/shaders/vertex.glsl` and `fragment.glsl` per spec Section 14 / brief Section 14. Create `apps/web/src/render/shader.ts` with `compileShader()`, `createProgram()`, `getUniformLocations()`, `getAttribLocations()` utility functions. Error handling: shader compile failure → throw with GLSL error log. Import shaders via Vite `?raw` suffix. |
| **Scope** | Shader files + compilation utilities. No draw calls, no buffer management. |
| **Files to Create** | `apps/web/src/render/shaders/vertex.glsl`, `apps/web/src/render/shaders/fragment.glsl`, `apps/web/src/render/shader.ts` |
| **Dependencies** | T-07 |
| **Definition of Done** | `tsc --noEmit` + `eslint` pass. `compileShader` compiles both shaders in a WebGL context. `createProgram` links them. Uniform/attribute locations retrieved. |
| **Test Criteria** | `__tests__/renderer.test.ts` (partial — shader compilation): mock WebGL context, verify compile/link calls. Verify error on bad GLSL. ~3 tests. |

---

### T-21: Scene Graph

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-21-scene-graph` |
| **Background** | The scene graph decouples domain data from the renderer (architecture Section 6). It manages VBO lifecycle, z-ordering, and provides the data the renderer iterates. |
| **Description** | Create `apps/web/src/scene/nodes.ts` with `SceneNode`, `LayerNode`, `BoardNode`, `OverlayGroup`, `OverlayNode`, `SceneRoot` interfaces per architecture Section 6.2. Create `apps/web/src/scene/scene.ts` with `SceneManager` class per architecture Section 6.3: `addLayer()` (upload VBOs, create LayerNode, insert by z-order), `clear()` (delete all VBOs), `getVisibleLayers()`, `getBounds()`, `releaseGPUResources()`, `restoreGPUResources()`. |
| **Scope** | Scene graph only. Does not own the WebGL context (receives it as constructor argument). |
| **Files to Create** | `apps/web/src/scene/nodes.ts`, `apps/web/src/scene/scene.ts` |
| **Dependencies** | T-07, T-20 |
| **Definition of Done** | `tsc --noEmit` + `eslint` pass. `addLayer` creates VBOs and inserts node in z-order. `clear` deletes all buffers. `getVisibleLayers` returns only visible nodes sorted by z-order. |
| **Test Criteria** | `__tests__/scene.test.ts`: add 3 layers → correct z-order. Toggle visibility → `getVisibleLayers` excludes hidden. `clear` → empty scene. `getBounds` → union of all layer bounds. ~8 tests (requires WebGL mock). |

---

### T-22: Renderer (Dirty-Flag Draw Loop)

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-22-renderer` |
| **Background** | The renderer implements on-demand drawing (architecture Section 9, Decision AD-002). It consumes the scene graph and view matrix to produce frames. |
| **Description** | Create `apps/web/src/render/renderer.ts` with `Renderer` class per architecture Section 9.1. Implement: `markDirty()` with rAF scheduling, `renderFrame()`, `draw()` sequence per architecture Section 9.3. WebGL state: enable blending (`SRC_ALPHA, ONE_MINUS_SRC_ALPHA`), clear color `#1a1a1a`, enable `OES_element_index_uint`. Per-layer: bind VBO, set `u_color` with opacity, set `u_viewMatrix`, `drawElements`. Handle canvas resize (update viewport). WebGL context lost/restored events per architecture Section 9.6. |
| **Scope** | `renderer.ts` only. Receives `SceneManager` and WebGL context as dependencies. |
| **Files to Create** | `apps/web/src/render/renderer.ts` |
| **Dependencies** | T-20, T-21 |
| **Definition of Done** | `tsc --noEmit` + `eslint` pass. `markDirty()` schedules exactly one rAF. Multiple `markDirty()` calls coalesce. `draw()` iterates visible layers, sets uniforms, calls `drawElements`. Context loss → error state. Context restore → re-upload + redraw. |
| **Test Criteria** | `__tests__/renderer.test.ts`: markDirty twice → one rAF. draw with 0 layers → no error. draw with mock layers → correct GL calls. Canvas resize → viewport updated. ~6 tests (WebGL mock). |

---

### T-23: View Matrix + Fit-to-View + Zoom + Pan

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-23-view-zoom-pan` |
| **Background** | All interaction ultimately updates the view matrix (architecture Section 9.4, 18.1-18.3). Zoom must be cursor-centered. Pan is a simple translation. Fit-to-view computes initial scale from bounds. |
| **Description** | Create `apps/web/src/interaction/zoom.ts` with cursor-centered zoom logic per architecture Section 18.1. Create `apps/web/src/interaction/pan.ts` with click-drag pan logic. Create `apps/web/src/interaction/interaction.ts` as the dispatcher: mouse wheel → zoom, mouse down+move → pan, keyboard `+`/`-`/`0` → zoom/fit, arrow keys → pan. Implement `computeViewMatrix(viewState, canvasW, canvasH, boardBounds)` per architecture Section 9.4. Implement `fitToView(bounds, canvasW, canvasH)` per architecture Section 18.3. Implement `screenToBoard()` and `boardToScreen()` per architecture Section 18.2. Wire events to `store.viewState` updates. |
| **Scope** | `interaction/` directory. Reads/writes store. Calls `renderer.markDirty()`. |
| **Files to Create** | `apps/web/src/interaction/zoom.ts`, `apps/web/src/interaction/pan.ts`, `apps/web/src/interaction/interaction.ts` |
| **Dependencies** | T-07, T-22 |
| **Definition of Done** | `tsc --noEmit` + `eslint` pass. Scroll → zoom centered on cursor. Click-drag → pan. `+`/`-` keys → zoom. `0` → fit-to-view. Arrow keys → pan. All boundary conditions BC-INT-001 through BC-INT-006 handled. |
| **Test Criteria** | `__tests__/zoom.test.ts`: zoom in → zoom increases, center adjusted. Zoom clamp at min/max. `__tests__/pan.test.ts`: pan by delta → center offset. Spec tests UT-TS-008 through UT-TS-011. ~10 tests. |

---

### T-24: Touch Gestures + Keyboard Shortcuts

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `feat/T-24-touch-keyboard` |
| **Background** | Mobile support is SHOULD priority (spec FR-504, FR-505). Keyboard shortcuts are SHOULD (FR-506). Both are input variations of the same zoom/pan logic. |
| **Description** | Create `apps/web/src/interaction/touch.ts`: pinch-to-zoom (two-finger gesture), single-finger drag-to-pan (with 50ms hold delay to avoid scroll conflict). Wire into `interaction.ts` dispatcher. Add all keyboard shortcuts from spec Section 18.2 (Tab, +/-, 0, arrows, Escape). |
| **Scope** | `interaction/touch.ts` + updates to `interaction.ts`. |
| **Files to Create** | `apps/web/src/interaction/touch.ts` |
| **Files to Change** | `apps/web/src/interaction/interaction.ts` |
| **Dependencies** | T-23 |
| **Definition of Done** | `tsc --noEmit` + `eslint` pass. Touch pinch changes zoom. Touch drag pans. All keyboard shortcuts from spec Section 18.2 functional. |
| **Test Criteria** | Manual testing on mobile device / Chrome DevTools mobile emulation. Keyboard shortcuts verified in E2E (deferred to T-29). |

---

## Phase 5 — UI

### T-25: Upload Zone UI

| Field | Detail |
|-------|--------|
| **Effort** | M |
| **Branch** | `feat/T-25-upload-zone` |
| **Background** | The upload zone is the first thing users see (spec FR-607). It handles drag-drop and file picker (FR-100, FR-101). Accessibility requires keyboard activation and ARIA labels (spec Section 18). |
| **Description** | Create `apps/web/src/ui/upload-zone.ts`. Render the centered upload prompt from wireframe (brief Section 17): dashed border, "Drop Gerber .zip or click to open" text. Wire: dragover/drop events → `zip-handler.extractAndIdentify()` → `workerClient.parseFiles()` → store updates. Wire: click → hidden `<input type="file" accept=".zip">`. Show visual feedback on drag-over (border highlight). Hide upload zone when `appState === "rendered"`. Re-show on re-upload (FR-107: clear previous state first). Tailwind styling, WCAG-compliant focus ring. |
| **Scope** | `upload-zone.ts` only. Reads/writes store. Calls WorkerClient and SceneManager. |
| **Files to Create** | `apps/web/src/ui/upload-zone.ts` |
| **Dependencies** | T-07, T-09, T-12 |
| **Definition of Done** | `tsc --noEmit` + `eslint` pass. Drag-drop works. File picker works. Visual feedback on hover/drag. Error messages for invalid files. Upload zone hides after successful load. Re-upload clears previous state. Accessible: keyboard navigable, `aria-label` on dropzone. |
| **Test Criteria** | Covered by E2E tests in T-29 (`upload.spec.ts`). |

---

### T-26: Layer Panel + Status Bar + Error Banner

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-26-layer-panel-ui` |
| **Background** | These three UI components subscribe to the store and render the sidebar, bottom bar, and error overlay (spec FR-600-FR-608, architecture Section 5.5). |
| **Description** | Create `apps/web/src/ui/layer-panel.ts`: render checkbox + color swatch + name per layer from `store.layers`. Toggle visibility on check/uncheck → `store.layers` update. Global opacity slider → `store.globalOpacity`. Use `<fieldset>`, `<legend>`, `<label>` for accessibility (spec Section 18.1). Create `apps/web/src/ui/status-bar.ts`: show layer count, shape count, warning count from `store.layers`. Show cursor coordinates from `store.cursorPosition`. Show board dimensions from `store.boardDimensions`. Create `apps/web/src/ui/error-banner.ts`: show `store.error` as red banner, dismissable via close button or Escape. Create `apps/web/src/ui/ui.ts` as the orchestrator that creates all UI components and subscribes them to the store. |
| **Scope** | `ui/` directory (4 files). |
| **Files to Create** | `apps/web/src/ui/layer-panel.ts`, `apps/web/src/ui/status-bar.ts`, `apps/web/src/ui/error-banner.ts`, `apps/web/src/ui/ui.ts` |
| **Dependencies** | T-07 |
| **Definition of Done** | `tsc --noEmit` + `eslint` pass. Layer panel shows correct layers with color swatches. Toggling checkbox hides/shows layer. Opacity slider works. Status bar shows stats. Error banner appears on error, dismissable. All WCAG 2.1 AA criteria from spec Section 18.1 met (semantic HTML, labels, contrast, focus visible). |
| **Test Criteria** | Covered by E2E tests in T-29 (`layers.spec.ts`, `error-states.spec.ts`). |

---

### T-27: Main Composition Root + Loading Indicator

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-27-composition-root` |
| **Background** | `main.ts` is the mediator that wires all modules together (architecture Section 17.2, pattern: Mediator). It's the only file that imports everything. The loading indicator (FR-605) bridges the store's loading state to a visible spinner. |
| **Description** | Rewrite `apps/web/src/main.ts` per architecture Section 17.2: create store, get canvas, init WebGL (+OES_element_index_uint), create SceneManager, create Renderer, create WorkerClient (await ready), setup interaction, setup UI (including upload zone, layer panel, status bar, error banner), wire upload flow (drop/click → zip-handler → worker → store → scene → render), wire subscriptions (viewState/opacity → markDirty). Add loading overlay: when `store.loadingProgress` changes, show spinner + "Parsing layer N of M..." text. Add cursor coordinate tracking: mousemove on canvas → `screenToBoard()` → `store.cursorPosition`. Update `index.html` with final DOM structure: header, sidebar, canvas, status bar. |
| **Scope** | `main.ts` rewrite + `index.html` finalization. |
| **Files to Change** | `apps/web/src/main.ts`, `apps/web/index.html` |
| **Dependencies** | T-12, T-22, T-23, T-25, T-26 |
| **Definition of Done** | Full pipeline works end-to-end: drop ZIP → layers parsed → board rendered → zoom/pan works → layer toggles work → status bar shows stats → coordinates update on mouse move. All FR-* requirements from spec Section 5 are functional. |
| **Test Criteria** | Manual E2E validation: drop a KiCad ZIP → board renders correctly with all layers. This is the "it works" milestone. Formal E2E tests in T-29. |

---

## Phase 6 — Testing & Polish

### T-28: TypeScript Unit Tests

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `test/T-28-ts-unit-tests` |
| **Background** | 90% TS coverage is required (spec NFR-501). Unit tests cover the modules that don't need a browser: signals, store, layer-identify, zip-handler, zoom math, pan math, scene graph. |
| **Description** | Create/complete all unit tests in `apps/web/__tests__/`: `signal.test.ts` (~5 tests), `store.test.ts` (~5 tests), `layer-identify.test.ts` (~20 tests), `zip-handler.test.ts` (~12 tests), `zoom.test.ts` (~5 tests), `pan.test.ts` (~3 tests), `scene.test.ts` (~8 tests), `renderer.test.ts` (~6 tests). Use Vitest. Mock JSZip and WebGL where needed. Verify coverage >= 90% lines via `@vitest/coverage-v8`. |
| **Scope** | `__tests__/` directory only. |
| **Files to Create** | All 8 test files listed above (some may already exist as stubs from earlier tasks). |
| **Dependencies** | T-07, T-08, T-09, T-21, T-22, T-23 |
| **Definition of Done** | `pnpm run test` passes. `pnpm run test:coverage` shows >= 90% line coverage, >= 80% branch coverage on `apps/web/src/`. All spec tests UT-TS-001 through UT-TS-014 pass. |
| **Test Criteria** | 64+ unit tests pass. Coverage gate met. |

---

### T-29: E2E Tests + Visual Regression + Accessibility

| Field | Detail |
|-------|--------|
| **Effort** | XL |
| **Branch** | `test/T-29-e2e-tests` |
| **Background** | E2E tests validate the full user experience across browsers (spec Section 16.6). Visual regression catches rendering bugs. Accessibility tests enforce WCAG 2.1 AA (spec Section 18). |
| **Description** | Implement all 6 Playwright spec files from spec Section 16.6.2: `upload.spec.ts` (5 cases), `rendering.spec.ts` (4 cases + visual regression screenshot), `interaction.spec.ts` (5 cases), `layers.spec.ts` (3 cases), `error-states.spec.ts` (3 cases), `accessibility.spec.ts` (3 cases with axe-core). Configure 3 browser projects (Chromium, Firefox, WebKit). Add `@axe-core/playwright` for WCAG checks. Capture reference screenshots for visual regression. All E2E fixtures used from `apps/web/e2e/fixtures/`. |
| **Scope** | `apps/web/e2e/tests/` (6 files). |
| **Files to Create** | `apps/web/e2e/tests/upload.spec.ts`, `apps/web/e2e/tests/rendering.spec.ts`, `apps/web/e2e/tests/interaction.spec.ts`, `apps/web/e2e/tests/layers.spec.ts`, `apps/web/e2e/tests/error-states.spec.ts`, `apps/web/e2e/tests/accessibility.spec.ts` |
| **Dependencies** | T-27, T-05 |
| **Definition of Done** | `pnpm run test:e2e` passes across Chromium, Firefox, WebKit. Visual regression screenshots committed. axe-core WCAG 2.1 AA scan passes. All 23 E2E test cases from spec Section 16.6.2 pass. |
| **Test Criteria** | 23 E2E tests pass × 3 browsers = 69 test runs. PERF-003 (< 2000ms upload→render) validated. |

---

### T-30: Service Worker + Performance Optimization

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `feat/T-30-sw-perf` |
| **Background** | Offline support (spec NFR-401) and bundle size gates (spec Section 17) are non-functional requirements that must be met before deployment. |
| **Description** | Create `apps/web/public/sw.js` per architecture Section 19.2: cache-first strategy, versioned cache, precache all assets. Register service worker in `main.ts`. Measure and optimize WASM binary size: verify < 800KB gzipped (PERF-005). Measure total bundle: verify < 1.5MB gzipped (PERF-006). If over budget: analyze with `twiggy`, adjust `opt-level`, strip debug info. Add size-checking script to CI (`check-bundle-size.sh`). Add Cloudflare `_headers` file with CSP per spec Section 23. |
| **Scope** | `public/sw.js`, `main.ts` update, CI script, `_headers` file. |
| **Files to Create** | `apps/web/public/sw.js`, `apps/web/public/_headers` |
| **Files to Change** | `apps/web/src/main.ts` (add SW registration), `.github/workflows/ci.yml` (add size gate step) |
| **Dependencies** | T-27 |
| **Definition of Done** | Page loads offline after first visit (verified in Playwright). WASM binary < 800KB gzipped. Total bundle < 1.5MB gzipped. CSP headers set. CI includes size gate checks. |
| **Test Criteria** | PERF-005 and PERF-006 pass. Manual: load page → disconnect network → reload → page works. |

---

### T-31: Deployment + README + Cross-Browser Testing

| Field | Detail |
|-------|--------|
| **Effort** | L |
| **Branch** | `chore/T-31-deployment` |
| **Background** | The project must be deployed to a public URL (brief Section 19, criteria #8). The README must have screenshots and architecture (criteria #9). Cross-browser testing validates the compatibility matrix (spec Section 19). |
| **Description** | Configure Cloudflare Pages: connect GitHub repo, set build command, set output directory. Update `.github/workflows/deploy.yml` with wrangler deployment. Write comprehensive README: project description, live demo link, screenshot(s) of rendered board, tech stack, architecture diagram, local development instructions, contributing guidelines, license. Test in all primary browsers from spec Section 19: Chrome 90+, Firefox 90+, Safari 15+, Edge 90+. Test on mobile (Chrome Android, Safari iOS). |
| **Scope** | Deployment config, README, manual cross-browser testing. |
| **Files to Change** | `README.md` (full rewrite), `.github/workflows/deploy.yml` (activate wrangler) |
| **Dependencies** | T-29, T-30 |
| **Definition of Done** | App live at public URL. README has: project name, live demo link, at least 1 screenshot, tech stack, architecture overview, dev setup instructions, license badge, CI badge. All 4 primary browsers render a KiCad board correctly. Success criteria 1-10 from brief Section 19 all pass. |
| **Test Criteria** | Manual: visit URL in Chrome, Firefox, Safari, Edge → board renders. All 10 success criteria verified and checked off. |

---

## Task Dependency Graph

```
T-00
 └── T-01
      ├── T-02
      │    └── T-03
      │         └── T-04
      ├── T-05
      └── T-06
           ├── T-10 ──────────────┐
           │    └── T-12 ──────┐  │
           ├── T-13            │  │
           │    └── T-14       │  │
           │         └── T-15  │  │
           ├── T-16            │  │
           ├── T-17            │  │
           │                   │  │
           └── T-11            │  │
                               │  │
T-03                           │  │
 └── T-07                     │  │
      ├── T-08                 │  │
      │    └── T-09            │  │
      ├── T-20                 │  │
      │    └── T-21            │  │
      │         └── T-22       │  │
      │              └── T-23  │  │
      │                   └── T-24
      └────────────────────────┘  │
                                  │
T-13+T-14+T-15+T-16+T-17+T-10 ──►T-18
                                   └── T-19
T-09+T-12 ──► T-25
T-07 ──► T-26
T-12+T-22+T-23+T-25+T-26 ──► T-27
T-07..T-23 ──► T-28
T-27+T-05 ──► T-29
T-27 ──► T-30
T-29+T-30 ──► T-31
```

---

## Summary

| Phase | Tasks | Total Effort |
|-------|-------|-------------|
| 0 — Foundation | T-00 through T-05 | S + M + M + L + M + M = ~12h |
| 1 — Core Types & Infrastructure | T-06 through T-09 | L + L + M + M = ~12h |
| 2 — Parsing Pipeline | T-10 through T-12 | M + L + L = ~10h |
| 3 — Geometry Engine | T-13 through T-19 | L + M + L + M + XL + L + L = ~30h |
| 4 — WebGL Rendering | T-20 through T-24 | M + L + L + L + M = ~16h |
| 5 — UI | T-25 through T-27 | M + L + L = ~10h |
| 6 — Testing & Polish | T-28 through T-31 | L + XL + L + L = ~24h |
| **Total** | **32 tasks** | **~114h** |

---

> **End of Task Breakdown**  
> **Next step:** Execute T-00.
