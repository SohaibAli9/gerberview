# GERBER PCB VIEWER — Agent Execution Brief

> **Purpose:** This document is the complete handoff for an AI coding agent (Cursor, Claude Code, etc.) to plan, architect, build, test, and deploy a browser-based Gerber PCB viewer from scratch. It contains the researched context, validated decisions, requirements, architecture, known pitfalls, and step-by-step execution plan. The human's role is steering and visual QA. The agent's role is everything else.

> **Date:** 2026-02-21 | **Author context:** Senior .NET engineer, zero Rust experience, strong distributed systems background.

---

## 1. PROJECT IDENTITY

**Name:** GerberView (working title)
**One-liner:** A fast, free, browser-native Gerber PCB viewer. No signup, no upload, no backend.
**Repo:** To be created on GitHub under user's account.
**License:** MIT
**Deployment:** Cloudflare Pages (free tier, $0/month, unlimited bandwidth)
**Domain:** TBD (e.g., gerberview.dev — ~$12/year)

---

## 2. WHAT IS THIS PROJECT

### The Problem
Every PCB ever manufactured uses Gerber files (RS-274X format) as the universal exchange format between designers and fabrication houses. Engineers need to review, share, and QC these files. Current options are all compromised:

| Viewer | Problem |
|--------|---------|
| Altium 365 | Proprietary, requires account, Altium-only ecosystem |
| KiCanvas | KiCad native format only — not Gerber |
| tracespace.io | JavaScript/SVG — creator abandoned it, admitted "JS and SVG are wrong tools for PCB rendering" |
| Gerbv | Desktop Linux app, 2008-era GTK2 UI, no web version |
| NextPCB/JLCPCB/PCBWay viewers | Free but tied to PCB manufacturer sales funnels, server-dependent |
| GerbLook | Basic, limited feature set |

### The Solution
A static website. User drops a .zip of Gerber files. Rust parses them (compiled to WASM). WebGL renders them on the GPU. Nothing leaves the browser. Loads fast, renders at 60fps, works offline after first visit.

### Strategic Purpose
This is NOT a commercial product (see Market Analysis below). It IS:
- A portfolio piece demonstrating Rust + WASM + WebGL + domain-specific engineering
- Career leverage for senior/staff engineering roles, Erasmus Mundus applications, and remote positions at systems companies
- An open-source contribution to the PCB tooling ecosystem
- A Rust learning vehicle with a tangible, visual output

### Market Reality (Researched)
PCB manufacturers (NextPCB, JLCPCB, PCBWay) offer free Gerber viewers as loss leaders subsidized by $2/board manufacturing revenue. You cannot compete with "free, backed by manufacturing revenue." Hobbyist user base (~59K on r/PrintedCircuitBoard) uses viewers for 2 minutes when needed. Realistic steady-state traffic: 500-1,500 MAU. Ad revenue at that scale: $3-9/month. **Monetization is not viable. This is a portfolio project.**

---

## 3. TECHNOLOGY STACK (VALIDATED)

```
┌─ Browser ──────────────────────────────────────────┐
│                                                     │
│  TypeScript/JS                                      │
│  ├── File upload (drag-drop + file picker)          │
│  ├── ZIP extraction (JSZip)                         │
│  ├── Layer identification (filename heuristics)     │
│  ├── UI controls (layer toggles, opacity sliders)   │
│  └── WebGL rendering pipeline                       │
│       ├── Shader programs (GLSL)                    │
│       ├── Buffer management                         │
│       └── View matrix (zoom/pan)                    │
│                                                     │
│  Rust → WASM (via wasm-pack + wasm-bindgen)         │
│  ├── gerber_parser crate (MakerPnP, v0.5.0)        │
│  ├── gerber-types crate (AST definitions)           │
│  ├── Custom geometry engine                         │
│  │   ├── Aperture expansion                         │
│  │   ├── Stroke widening (D01 draws → quads)        │
│  │   ├── Arc tessellation                           │
│  │   ├── Region triangulation                       │
│  │   └── Polarity handling                          │
│  └── Vertex buffer output (Float32Array)            │
│                                                     │
└─────────────────────────────────────────────────────┘

Hosting: Cloudflare Pages (static files, free)
CI/CD: GitHub Actions
```

### Why Each Choice

| Choice | Reason |
|--------|--------|
| Rust for parsing | gerber_parser crate exists and is mature. Compiles to WASM. Memory-safe. |
| WASM | Near-native speed in browser. No server needed. |
| WebGL (not Canvas2D) | GPU-accelerated. Handles 10K+ shapes at 60fps. Canvas2D chokes on dense boards. |
| TypeScript for UI | Standard web UI layer. Handles DOM, events, WebGL context. |
| wasm-pack + wasm-bindgen | Standard Rust→WASM toolchain. Well-documented, stable. |
| JSZip | Mature ZIP extraction library for browser. |
| Cloudflare Pages | Free, unlimited bandwidth, global CDN, zero config. |

---

## 4. DATA FLOW (VALIDATED)

```
User drops board.zip
    │
    ▼
[JS] Extract .zip → list of files with names + content
    │
    ▼
[JS] Identify layer type per file (filename heuristics)
     ├── *.F_Cu.gbr → top copper
     ├── *.B_Cu.gbr → bottom copper
     ├── *.F_Mask.gbr → top solder mask
     ├── *.B_Mask.gbr → bottom solder mask
     ├── *.F_SilkS.gbr → top silkscreen
     ├── *.B_SilkS.gbr → bottom silkscreen
     ├── *.Edge_Cuts.gbr → board outline
     ├── *.drl / *.xln → drill (Excellon)
     └── (patterns vary by CAD tool — KiCad, Eagle, Altium, etc.)
    │
    ▼
[JS→WASM] Pass file bytes to Rust parse function
    │
    ▼
[Rust/WASM] gerber_parser::parse(bytes) → GerberDoc (typed AST)
    │
    ▼
[Rust/WASM] Geometry engine converts AST → vertex buffers
     ├── D03 flash → positioned shape vertices
     ├── D01 draw → stroke-expanded quad strip
     ├── G02/G03 arc → tessellated line segments → quad strip
     ├── G36/G37 region → polygon boundary → triangulated mesh
     └── Aperture macros → compound shape vertices
    │
    ▼
[WASM→JS] Return Float32Array (positions) + metadata (bounds, layer info)
    │
    ▼
[JS/WebGL] Upload vertex buffers to GPU
     ├── Create VBO per layer
     ├── Set per-layer color uniform
     ├── Set view matrix uniform (zoom/pan state)
     └── Draw calls per visible layer
    │
    ▼
[WebGL] GPU renders all layers composited on dark background
    │
    ▼
[JS] User interaction loop:
     ├── Scroll → update zoom in view matrix → re-render
     ├── Click-drag → update pan in view matrix → re-render
     ├── Layer toggle → enable/disable draw call → re-render
     └── Opacity slider → update alpha uniform → re-render
```

---

## 5. EXISTING CRATES & REFERENCES (USE THESE)

### Must-Use Crates

| Crate | Version | What it does | WASM-compatible? |
|-------|---------|-------------|-----------------|
| `gerber_parser` | 0.5.0 | Parses RS-274X Gerber files → typed AST | ✅ Yes (pure Rust) |
| `gerber-types` | (re-exported via gerber_parser) | Type definitions for all Gerber commands | ✅ Yes |
| `wasm-bindgen` | latest | Rust↔JS interop | ✅ (it IS the bridge) |
| `web-sys` | latest | Browser API bindings (WebGL context, etc.) | ✅ |
| `js-sys` | latest | JavaScript type bindings | ✅ |

### Do NOT Use
| Crate | Why not |
|-------|---------|
| `gerber-viewer` | Coupled to egui rendering. Cannot compile to WASM as-is. **BUT: study its source code for geometry conversion logic.** |
| `egui` | Desktop GUI framework. Not the rendering target. |

### Reference Implementations to Study
| Project | URL | What to learn |
|---------|-----|---------------|
| MakerPnP gerber-viewer | https://github.com/MakerPnP/gerber-viewer | Geometry conversion pipeline (Gerber AST → shapes). THE reference for how each Gerber command maps to renderable geometry. |
| tracespace (v5) | https://github.com/tracespace/tracespace | Layer identification by filename (identify-layers package). Test fixtures with real-world Gerber filenames from all major CAD tools. |
| gerbonara (Python) | https://gerbolyze.gitlab.io/gerbonara/ | Clean API design for Gerber abstraction. Shows how to represent Line, Arc, Region as graphical objects. |
| LogRocket Rust+WASM+WebGL tutorial | https://blog.logrocket.com/implement-webassembly-webgl-viewer-using-rust/ | Complete working example of Rust→WASM→WebGL pipeline with wasm-pack. |
| wasm-bindgen WebGL example | (in wasm-bindgen guide) | Official WebGL rendering from Rust/WASM. |
| Gerber RS-274X spec | https://www.ucamco.com/files/downloads/file/81/the_gerber_file_format_specification.pdf | The actual spec. ~200 pages but only ~50 pages of core content. |

### Rust Coding Standards
| Resource | URL | Usage |
|----------|-----|-------|
| Microsoft Pragmatic Rust Guidelines (agent-optimized) | https://microsoft.github.io/rust-guidelines/agents/all.txt | **DROP THIS INTO AGENT INSTRUCTIONS.** Condensed format designed for AI coding assistants. Covers error handling, API design, testing, documentation. |
| Microsoft Pragmatic Rust Guidelines (human) | https://microsoft.github.io/rust-guidelines/ | Full readable version for the human developer. |

---

## 6. FUNCTIONAL REQUIREMENTS (MVP)

| ID | Requirement | Priority | Notes |
|----|-------------|----------|-------|
| F1 | Upload .zip containing Gerber + drill files via drag-drop or file picker | MUST | Single entry point. Accept .zip, .rar not needed. |
| F2 | Auto-identify layer types from filenames | MUST | Support KiCad, Eagle, Altium, EasyEDA naming conventions minimum. Use tracespace's identify-layers as reference. |
| F3 | Parse all Gerber RS-274X files in the zip | MUST | Via gerber_parser crate. Handle: apertures (C/R/O/P), D-codes (D01/D02/D03), regions (G36/G37), arcs (G02/G03), polarity (LPD/LPC), step-repeat (SR), block apertures (AB), aperture macros (AM). |
| F4 | Parse Excellon drill files | MUST | Simpler format. Tool definitions + hole coordinates. |
| F5 | Convert parsed Gerber to renderable geometry | MUST | The core engineering challenge. See Geometry Pipeline section. |
| F6 | Render layers via WebGL on dark background | MUST | GPU-accelerated. 60fps target. |
| F7 | Color-code layers by type | MUST | Top copper: red. Bottom copper: blue. Top mask: green (transparent). Bottom mask: green (transparent). Silkscreen: white. Drill: yellow circles. Outline: gray. |
| F8 | Toggle layer visibility | MUST | Checkbox per layer in sidebar. |
| F9 | Zoom via scroll wheel | MUST | Centered on cursor position. |
| F10 | Pan via click-drag | MUST | Standard map-style panning. |
| F11 | Fit-to-view on initial load | MUST | Calculate bounding box of all layers, set zoom to fit. |
| F12 | Layer opacity control | SHOULD | Slider per layer or global. |
| F13 | Display board dimensions | SHOULD | Read from outline layer bounding box. |
| F14 | Cursor coordinate display | SHOULD | Show X,Y position in board units (mm/inches). |
| F15 | Keyboard shortcuts | COULD | +/- for zoom, 0 for fit-to-view. |

### Explicitly OUT of MVP
- Measurement tools (ruler/distance)
- DFM (Design for Manufacturing) checks
- 3D rendering
- Gerber X2 metadata display
- Net highlighting
- BOM integration
- Export to PNG/SVG
- Excellon routing (only drill holes)
- Multi-board/panel support
- File editing/modification

---

## 7. NON-FUNCTIONAL REQUIREMENTS

| ID | Requirement | Target | Rationale |
|----|-------------|--------|-----------|
| NF1 | Parse + render time (typical 6-layer board) | < 2 seconds | Competitive with existing viewers |
| NF2 | Render framerate during interaction | 60fps | WebGL should achieve this easily |
| NF3 | WASM binary size (gzipped) | < 500KB | Fast initial load |
| NF4 | Total page weight (gzipped) | < 1MB | Including JS, CSS, WASM |
| NF5 | Works offline after first load | Yes | Service worker + cache |
| NF6 | Data privacy | Zero data leaves browser | All processing client-side, no telemetry |
| NF7 | Browser support | Chrome 90+, Firefox 90+, Safari 15+, Edge 90+ | WebGL 1.0 baseline |
| NF8 | Mobile support | Functional (touch zoom/pan) | Desktop-first, mobile-acceptable |
| NF9 | Hosting cost | $0/month | Cloudflare Pages free tier |
| NF10 | Accessibility | Keyboard navigable, high-contrast defaults | Dark background is default |
| NF11 | Error handling | Graceful degradation on malformed Gerber | Show what can be parsed, warn on errors |

---

## 8. ARCHITECTURE

### Project Structure
```
gerberview/
├── .github/
│   └── workflows/
│       ├── ci.yml              # Lint + test + build on every PR
│       └── deploy.yml          # Build + deploy to Cloudflare on main push
├── rust/                       # Rust/WASM crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs              # WASM entry points (#[wasm_bindgen] exports)
│   │   ├── geometry/
│   │   │   ├── mod.rs
│   │   │   ├── aperture.rs     # Aperture → shape conversion
│   │   │   ├── stroke.rs       # D01 draw → stroke-expanded geometry
│   │   │   ├── arc.rs          # Arc interpolation + tessellation
│   │   │   ├── region.rs       # Region fill → triangulation
│   │   │   ├── triangulate.rs  # Ear-clipping or similar triangulation
│   │   │   └── types.rs        # VertexBuffer, BoundingBox, LayerGeometry
│   │   ├── excellon/
│   │   │   ├── mod.rs
│   │   │   └── parser.rs       # Excellon drill file parser
│   │   └── layer_id.rs         # Filename → layer type identification
│   └── tests/
│       ├── parse_tests.rs      # Gerber parsing integration tests
│       ├── geometry_tests.rs   # Geometry conversion tests
│       ├── excellon_tests.rs   # Drill file parsing tests
│       └── fixtures/           # Real Gerber files for testing
│           ├── simple-board/   # Minimal test board
│           ├── kicad-board/    # KiCad-exported board
│           └── eagle-board/    # Eagle-exported board
├── web/                        # Frontend
│   ├── index.html
│   ├── src/
│   │   ├── main.ts             # Entry point
│   │   ├── viewer.ts           # WebGL rendering pipeline
│   │   ├── shaders/
│   │   │   ├── vertex.glsl     # Vertex shader
│   │   │   └── fragment.glsl   # Fragment shader
│   │   ├── interaction.ts      # Zoom/pan/touch handlers
│   │   ├── ui.ts               # Layer panel, upload zone, controls
│   │   └── zip-handler.ts      # ZIP extraction + file routing
│   ├── styles/
│   │   └── main.css
│   ├── package.json
│   └── vite.config.ts          # Or similar bundler
├── README.md
├── LICENSE                     # MIT
├── .gitignore
├── rustfmt.toml                # Rust formatting config
└── clippy.toml                 # Rust linting config
```

### Module Responsibilities

**`rust/src/lib.rs`** — WASM bridge. Exposes two functions:
```rust
#[wasm_bindgen]
pub fn parse_gerber(data: &[u8]) -> JsValue; // Returns LayerGeometry as JSON or typed struct

#[wasm_bindgen]
pub fn parse_excellon(data: &[u8]) -> JsValue; // Returns drill holes as JSON
```

**`rust/src/geometry/`** — THE core module. Takes gerber-types AST, outputs vertex buffers.
- Input: `Vec<GerberCommand>` from gerber_parser
- Output: `LayerGeometry { positions: Vec<f32>, indices: Vec<u32>, bounds: BoundingBox }`
- This is where 60% of the Rust code lives.

**`web/src/viewer.ts`** — WebGL renderer.
- Creates WebGL context from canvas
- Compiles shaders (simple: position + color uniform + alpha uniform)
- Manages VBOs per layer
- Handles draw loop: for each visible layer, bind VBO, set uniforms, drawElements
- View matrix: 3x3 affine transform (translate + scale) updated by interaction.ts

**`web/src/interaction.ts`** — Input handling.
- Scroll → zoom (centered on cursor)
- Mouse down + move → pan
- Touch pinch → zoom
- Touch drag → pan
- Updates view matrix, triggers re-render

---

## 9. GEOMETRY PIPELINE GUIDE

This is the hardest part of the project. The agent MUST understand these conversions.

### 9.1 Aperture Flash (D03)
A D03 command places an aperture shape at a coordinate.
```
D10*           // Select aperture D10 (e.g., Circle diameter 0.5mm)
X1000Y2000D03* // Flash at (1mm, 2mm)
```
**Conversion:** Look up aperture D10's shape definition. Generate vertices for that shape centered at (1, 2). Circle → n-gon (e.g., 32 segments). Rectangle → 4 vertices + 2 triangles. Obround → rectangle body + two semicircle endcaps.

### 9.2 Linear Draw (D01 with G01)
A D01 moves from current position to target while "drawing" with the current aperture.
```
G01*           // Linear interpolation mode
D10*           // Circle aperture, diameter 0.5mm
X1000Y1000D02* // Move to (1,1) without drawing
X5000Y3000D01* // Draw line from (1,1) to (5,3)
```
**Conversion:** This creates a thick line (width = aperture diameter). Expand into a quad strip:
1. Calculate line direction vector
2. Calculate perpendicular offset (half aperture width)
3. Generate 4 corner vertices (2 on each side of the line)
4. Add rounded endcaps if aperture is circular (semicircles at each end)
5. Output 2 triangles for the body, n triangles for each endcap

### 9.3 Circular Arc (D01 with G02/G03)
```
G75*           // Multi-quadrant arc mode
G02*           // Clockwise arc
X3000Y1000I1000J0D01* // Arc to (3,1) with center offset (1,0) from start
```
**Conversion:** 
1. Calculate arc center from current position + I,J offset
2. Calculate start angle and end angle
3. Tessellate: generate N points along the arc (N based on arc length, typically 32-128)
4. For each segment between consecutive points, expand into a thick line quad (same as 9.2)
5. Join segments smoothly

### 9.4 Region Fill (G36/G37)
```
G36*           // Begin region
X1000Y1000D02* // Move to start
X5000Y1000D01* // Line to...
X5000Y5000D01* // Line to...
X1000Y5000D01* // Line to...
X1000Y1000D01* // Close polygon
G37*           // End region — fill the polygon
```
**Conversion:**
1. Collect all boundary points (including arcs tessellated to line segments)
2. Form closed polygon
3. Triangulate using ear-clipping algorithm (or similar)
4. Output triangle list

### 9.5 Polarity (LPD/LPC)
- **LPD (Dark):** Normal drawing. Adds to the image.
- **LPC (Clear):** Subtractive. Removes from the image (like erasing).

**WebGL approach (two options):**
- Option A (simpler): Render dark polarity normally. Render clear polarity using stencil buffer to "cut out" regions.
- Option B (simpler still for MVP): Render clear polarity in background color. Works visually but incorrect for overlapping layers.
- **RECOMMENDATION FOR MVP:** Option B. Implement Option A later.

### 9.6 Step-Repeat (SR)
```
%SRX3Y2I5.0J4.0*%  // Repeat 3×2 grid, spacing 5mm × 4mm
... gerber commands ...
%SR*%               // End step-repeat
```
**Conversion:** Generate geometry for the block once, then duplicate vertices with X/Y offsets for each repeat position. Pure vertex buffer duplication.

### 9.7 Aperture Macros (AM)
Complex apertures defined by primitives (circles, lines, outlines, polygons) with arithmetic expressions.
```
%AMTHERMAL*
1,1,0.060,0,0*           // Circle, exposure on, diameter 0.060, at 0,0
1,0,0.030,0,0*           // Circle, exposure off (clear), diameter 0.030, at 0,0
20,1,0.005,0,-0.040,0,0.040,0* // Line, creating thermal spoke
20,1,0.005,-0.040,0,0.040,0,0* // Line, creating thermal spoke
%
```
**Conversion:** Evaluate macro primitives in order. Each primitive adds or clears geometry. gerber_parser already parses these. The geometry engine must handle the primitive-by-primitive composition.

---

## 10. TESTING STRATEGY

### Test Levels

**Unit Tests (Rust):**
- Aperture expansion: given aperture def, assert correct vertex count and positions
- Stroke widening: given line segment + aperture width, assert quad vertices
- Arc tessellation: given center/start/end angles, assert points lie on arc within tolerance
- Region triangulation: given polygon boundary, assert valid triangle mesh (no inversions, complete coverage)
- Excellon parsing: given drill file text, assert correct tool definitions and hole positions

**Integration Tests (Rust):**
- Parse real Gerber file → geometry → assert bounding box matches expected
- Parse full board .zip → assert all layers identified and parsed without errors
- Round-trip: parse known simple Gerber → geometry → assert vertex count matches expected shape count

**Visual Regression Tests (Browser):**
- Render known boards → screenshot → compare against reference images
- This catches rendering regressions that unit tests miss
- Tool: Playwright or Puppeteer for screenshot capture

**Performance Tests:**
- Parse + render benchmark on a complex board (e.g., Arduino Uno Gerbers)
- Assert: < 2 seconds total, < 500ms for parsing alone

### Test Fixtures (Real Data)

These are open-source PCB designs with freely available Gerber files:

| Board | Source | Complexity | Use for |
|-------|--------|-----------|---------|
| Arduino Uno | arduino.cc (open hardware) | Medium — 2-layer, moderate density | Primary integration test |
| Adafruit Feather | adafruit.com (open hardware) | Medium — 2-layer, SMD-heavy | SMD pad testing |
| KiCad demo board | KiCad installation includes sample project | Simple — good for unit tests | Basic functionality |
| tracespace fixtures | https://github.com/tracespace/tracespace/tree/v5/packages/fixtures | Various — multiple CAD tools | Filename identification testing |
| SparkFun boards | sparkfun.com (open hardware) | Various | Eagle-format Gerber testing |

**IMPORTANT:** The agent should download these during setup and include them in the test fixtures directory.

### Linting & Formatting

**Rust:**
```toml
# rustfmt.toml
edition = "2021"
max_width = 100
tab_spaces = 4
```
- `cargo fmt --check` in CI
- `cargo clippy -- -D warnings` in CI (all warnings are errors)
- Follow Microsoft Pragmatic Rust Guidelines

**TypeScript:**
- ESLint with strict config
- Prettier for formatting
- `strict: true` in tsconfig.json

### Logging
**Rust/WASM:**
- Use `web_sys::console::log_1()` for WASM console output
- Log: parse start/end times, command count, vertex count, errors/warnings
- Structured: `[GerberView] Parsed layer top_copper: 4,217 commands → 18,432 vertices in 43ms`

**TypeScript:**
- Console groups per operation
- Performance.mark/measure for timing
- Error boundary: catch and display parse failures gracefully in UI

---

## 11. CI/CD PIPELINE

### GitHub Actions: CI (`.github/workflows/ci.yml`)
Triggers: Every push and PR to main.

```yaml
# Steps:
# 1. Checkout
# 2. Install Rust toolchain + wasm32-unknown-unknown target
# 3. Install wasm-pack
# 4. cargo fmt --check
# 5. cargo clippy -- -D warnings
# 6. cargo test (unit + integration tests)
# 7. wasm-pack build --target web (verify WASM compilation)
# 8. Install Node.js
# 9. npm ci (in web/)
# 10. npm run lint
# 11. npm run build (verify frontend builds)
# 12. npm run test (if browser tests exist)
```

### GitHub Actions: Deploy (`.github/workflows/deploy.yml`)
Triggers: Push to main only.

```yaml
# Steps:
# 1-11 same as CI
# 12. Copy WASM output to web/public/
# 13. npm run build (production)
# 14. Deploy web/dist/ to Cloudflare Pages via wrangler
```

---

## 12. EXECUTION PLAN (PHASED)

### Phase 0: Project Setup
- [ ] Create GitHub repo with README, LICENSE (MIT), .gitignore
- [ ] Initialize Rust crate with Cargo.toml (dependencies: gerber_parser, wasm-bindgen, web-sys, js-sys)
- [ ] Initialize web/ with Vite + TypeScript
- [ ] Set up wasm-pack build pipeline
- [ ] Set up GitHub Actions CI
- [ ] Download test fixture Gerber files (Arduino Uno, KiCad sample)
- [ ] Verify: `wasm-pack build` succeeds with empty lib.rs
- [ ] Verify: `npm run dev` serves a page that loads WASM module
- [ ] **Commit: "Initial project scaffold with Rust/WASM/TS toolchain"**

### Phase 1: Parsing Pipeline
- [ ] Implement `parse_gerber()` WASM export that takes bytes, calls gerber_parser, returns command count
- [ ] Write unit tests: parse known Gerber file → assert command types and counts
- [ ] Implement Excellon drill parser (simpler — tool definitions + coordinates)
- [ ] Write unit tests for Excellon parsing
- [ ] Implement layer identification from filenames (port tracespace identify-layers logic)
- [ ] Write unit tests with tracespace's fixture data
- [ ] Verify from browser: upload file → console shows parse results
- [ ] **Commit: "Gerber and Excellon parsing via WASM with layer identification"**

### Phase 2: Geometry Engine (THE HARD PART)
- [ ] Define `LayerGeometry` struct: positions (Vec<f32>), indices (Vec<u32>), bounds
- [ ] Implement circle aperture → n-gon vertex generation
- [ ] Implement rectangle aperture → quad vertex generation
- [ ] Implement obround aperture → rect + semicircle endcaps
- [ ] Implement D03 flash → positioned shape
- [ ] Write tests: flash circle at (1,1) → assert 32 vertices around (1,1)
- [ ] Implement D01 linear draw → stroke-expanded quad strip
- [ ] Write tests: draw line with circle aperture → assert quad + endcaps
- [ ] Implement arc tessellation (G02/G03, multi-quadrant mode)
- [ ] Write tests: 90° arc → assert points on arc within tolerance
- [ ] Implement region fill (G36/G37) → ear-clipping triangulation
- [ ] Write tests: square region → 2 triangles, L-shape region → correct triangulation
- [ ] Implement polarity handling (LPD/LPC) — MVP approach: clear = background color
- [ ] Implement step-repeat (SR) — vertex buffer duplication with offsets
- [ ] Implement aperture macros — compose primitives
- [ ] Integration test: parse Arduino Uno copper layer → geometry → assert non-zero vertices, reasonable bounds
- [ ] **Commit: "Geometry engine: full Gerber command → vertex buffer pipeline"**

### Phase 3: WebGL Rendering
- [ ] Set up WebGL context from canvas element
- [ ] Write vertex shader (position + view matrix transform)
- [ ] Write fragment shader (flat color + alpha)
- [ ] Implement buffer upload: take Float32Array from WASM, create VBO
- [ ] Render single layer (hardcoded color)
- [ ] Implement per-layer color assignment based on layer type
- [ ] Implement view matrix for zoom/pan
- [ ] Implement mouse wheel zoom (centered on cursor)
- [ ] Implement click-drag pan
- [ ] Implement fit-to-view (calculate bounding box → set initial view matrix)
- [ ] Visual test: render Arduino Uno board → compare to reference viewer (KiCad GerbView)
- [ ] **Commit: "WebGL multi-layer rendering with zoom/pan"**

### Phase 4: UI & Integration
- [ ] Build upload zone (centered in canvas, drag-drop + click)
- [ ] Build layer panel (sidebar with checkboxes, color swatches)
- [ ] Wire layer toggles to WebGL draw calls
- [ ] Add opacity slider (per-layer or global)
- [ ] Add cursor coordinate display
- [ ] Add board dimensions display
- [ ] Add loading indicator during parse
- [ ] Add error display for malformed files
- [ ] Implement touch zoom/pan for mobile
- [ ] Style: dark theme, clean typography, minimal UI
- [ ] **Commit: "Complete UI with layer controls and interaction"**

### Phase 5: Polish & Deploy
- [ ] Add service worker for offline support
- [ ] Optimize WASM binary size (wasm-opt, strip debug symbols)
- [ ] Performance test: ensure < 2s load for Arduino Uno
- [ ] Write README with screenshots, usage instructions, tech stack, architecture diagram
- [ ] Set up Cloudflare Pages deployment
- [ ] Deploy to production URL
- [ ] Test across browsers (Chrome, Firefox, Safari, Edge)
- [ ] **Commit: "Production deployment with README and cross-browser testing"**

### Phase 6: Launch
- [ ] Post to Hacker News ("Show HN: I built a Gerber PCB viewer in Rust+WASM+WebGL")
- [ ] Post to r/PrintedCircuitBoard, r/rust, r/webdev
- [ ] Post to EE Twitter/Mastodon
- [ ] Pin repo on GitHub profile

---

## 13. KNOWN RISKS & CAVEATS

### Must Research Further
| Risk | Severity | Mitigation |
|------|----------|-----------|
| gerber_parser WASM compatibility | HIGH — untested | First task in Phase 0: verify `cargo build --target wasm32-unknown-unknown` with gerber_parser dependency. If it fails, check for std::fs or OS-dependent code. May need to fork and patch. |
| Arc interpolation edge cases | MEDIUM | Single-quadrant arcs (G74, deprecated since 2021) may appear in old files. Multi-quadrant (G75) is standard. Start with G75 only. Test with real files to see if G74 appears. |
| Ear-clipping triangulation correctness | MEDIUM | Self-intersecting polygons from malformed Gerbers can break triangulation. Use a robust library (e.g., port earcut algorithm) or handle gracefully with error fallback. |
| WASM memory limits | LOW | Large boards might generate millions of vertices. WASM has a ~4GB memory limit by default. Monitor memory usage. Implement geometry decimation if needed. |
| WebGL 1.0 vs 2.0 | LOW | Target WebGL 1.0 for maximum compatibility. WebGL 2.0 adds instanced rendering (useful for pad arrays) but isn't needed for MVP. |
| Touch interaction quality | LOW | Touch zoom/pan is notoriously finicky. Start with basic implementation, iterate. |

### Known Limitations of gerber_parser
From MakerPnP gerber-viewer README (as of Dec 2025):
- ❌ Thermal primitive (aperture macro primitive) — not supported
- ❌ Exposure (clear polarity in renderer) — only additive rendering
- ❌ Single quadrant arc mode (G74) — deprecated since 2021
- ❌ Image polarity (IP) — deprecated since 2013
- ❌ Various deprecated features (MI, OF, SF, IR, LN)

These limitations are acceptable for MVP. Modern Gerber files from KiCad, Eagle, Altium won't use deprecated features.

### Agent-Specific Pitfalls
| Pitfall | How to handle |
|---------|--------------|
| Agent may struggle with Rust borrow checker | Load Microsoft Pragmatic Rust Guidelines into agent context. Use `clone()` liberally at first, optimize later. |
| Agent may generate incorrect WebGL state management | Always bind VAO/VBO before drawing. Always unbind after. Check gl.getError() during development. |
| Agent may get arc math wrong | Provide explicit formulas. Test against known arcs. Compare visually to KiCad GerbView. |
| Agent may over-engineer the geometry pipeline | Enforce: get the simplest version working first (circles only), then add rectangle, then obround, then arcs, then regions. Incremental. |
| Agent may forget WASM memory patterns | All data crossing WASM↔JS boundary must be serialized (JSON) or shared via typed arrays (Float32Array). No passing Rust structs directly. |

---

## 14. WEBGL SHADER CODE (REFERENCE)

### Vertex Shader (simple 2D)
```glsl
attribute vec2 a_position;
uniform mat3 u_viewMatrix;  // 3x3 affine: translate + scale

void main() {
    vec3 pos = u_viewMatrix * vec3(a_position, 1.0);
    gl_Position = vec4(pos.xy, 0.0, 1.0);
}
```

### Fragment Shader
```glsl
precision mediump float;
uniform vec4 u_color;  // RGBA, alpha for layer opacity

void main() {
    gl_FragColor = u_color;
}
```

These are the MVP shaders. No lighting, no textures, no 3D. Just flat colored triangles with alpha blending.

### WebGL Setup Pseudocode
```
Enable blending: gl.enable(gl.BLEND)
Blend func: gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)
Clear color: gl.clearColor(0.1, 0.1, 0.1, 1.0)  // dark background

Per frame:
  gl.clear(COLOR_BUFFER_BIT)
  for each visible layer (back to front):
    gl.bindBuffer(layer.vbo)
    gl.uniform4fv(colorLoc, layer.color)
    gl.uniformMatrix3fv(viewMatrixLoc, viewMatrix)
    gl.drawElements(gl.TRIANGLES, layer.indexCount, gl.UNSIGNED_INT, 0)
```

---

## 15. LAYER COLOR SCHEME

| Layer Type | Color (RGBA) | Hex |
|-----------|-------------|-----|
| Top Copper | (0.8, 0.2, 0.2, 0.9) | #CC3333 |
| Bottom Copper | (0.2, 0.2, 0.8, 0.9) | #3333CC |
| Top Solder Mask | (0.1, 0.5, 0.1, 0.5) | #1A801A (50% alpha) |
| Bottom Solder Mask | (0.1, 0.5, 0.1, 0.5) | #1A801A (50% alpha) |
| Top Silkscreen | (0.9, 0.9, 0.9, 0.9) | #E6E6E6 |
| Bottom Silkscreen | (0.7, 0.7, 0.9, 0.9) | #B3B3E6 |
| Board Outline | (0.6, 0.6, 0.6, 1.0) | #999999 |
| Drill Holes | (0.9, 0.9, 0.2, 1.0) | #E6E633 |
| Top Paste | (0.8, 0.8, 0.8, 0.5) | #CCCCCC (50% alpha) |
| Bottom Paste | (0.8, 0.8, 0.8, 0.5) | #CCCCCC (50% alpha) |

---

## 16. LAYER IDENTIFICATION RULES

File extension and name patterns by CAD tool:

### KiCad
| Pattern | Layer |
|---------|-------|
| `*-F_Cu.gbr` or `*.GTL` | Top copper |
| `*-B_Cu.gbr` or `*.GBL` | Bottom copper |
| `*-F_Mask.gbr` or `*.GTS` | Top solder mask |
| `*-B_Mask.gbr` or `*.GBS` | Bottom solder mask |
| `*-F_SilkS.gbr` or `*.GTO` | Top silkscreen |
| `*-B_SilkS.gbr` or `*.GBO` | Bottom silkscreen |
| `*-Edge_Cuts.gbr` or `*.GKO` | Board outline |
| `*.drl` or `*.xln` | Drill file |
| `*-F_Paste.gbr` or `*.GTP` | Top paste |
| `*-B_Paste.gbr` or `*.GBP` | Bottom paste |

### Eagle
| Pattern | Layer |
|---------|-------|
| `*.cmp` or `*.top` | Top copper |
| `*.sol` or `*.bot` | Bottom copper |
| `*.stc` or `*.tsp` | Top solder mask |
| `*.sts` or `*.bsp` | Bottom solder mask |
| `*.plc` or `*.tsk` | Top silkscreen |
| `*.pls` or `*.bsk` | Bottom silkscreen |
| `*.bor` or `*.dim` | Board outline |
| `*.drl` or `*.drd` | Drill file |

### Altium
| Pattern | Layer |
|---------|-------|
| `*.GTL` | Top copper |
| `*.GBL` | Bottom copper |
| `*.GTS` | Top solder mask |
| `*.GBS` | Bottom solder mask |
| `*.GTO` | Top silkscreen |
| `*.GBO` | Bottom silkscreen |
| `*.GKO` or `*.GM1` | Board outline |
| `*.DRL` or `*.TXT` | Drill file |

### Fallback (Protel-style extensions)
Same as Altium — this is the most common convention.

**Implementation:** Try exact match first, then pattern match, then file content inspection (look for `%FSLAX*%` header for Gerber, `M48` for Excellon).

---

## 17. WIREFRAME

```
┌─────────────────────────────────────────────────────────────┐
│  ⚡ GerberView           [Fit] [Zoom+] [Zoom-]    [GitHub] │
├─────────────┬───────────────────────────────────────────────┤
│  LAYERS     │                                               │
│             │                                               │
│  ☑ ■ Top Cu │                                               │
│  ☑ ■ Bot Cu │                                               │
│  ☑ ■ Top Mk │           ┌─────────────────────┐             │
│  ☑ ■ Bot Mk │           │                     │             │
│  ☑ ■ Top Sk │           │   DROP GERBER .ZIP   │             │
│  ☑ ■ Bot Sk │           │   OR CLICK TO OPEN   │             │
│  ☑ ■ Drill  │           │                     │             │
│  ☑ ■ Outline│           └─────────────────────┘             │
│             │                                               │
│ ─────────── │                                               │
│ Opacity     │                                               │
│ ──────●──── │                                               │
│             │                                               │
│ Board Size: │                                               │
│ 68.6 × 53.3 │              X: 12.4mm  Y: 8.7mm            │
│ mm          │                                               │
├─────────────┴───────────────────────────────────────────────┤
│  Parsed 7 layers · 24,891 shapes · 0 warnings    [v0.1.0]  │
└─────────────────────────────────────────────────────────────┘
```

**States:**
1. **Initial:** Upload prompt centered. Layer panel empty/hidden.
2. **Loading:** Spinner + progress text ("Parsing top copper... 3/7 layers")
3. **Rendered:** PCB visible. Layer panel populated. Status bar shows stats.
4. **Error:** Red banner with error message. Partial render if possible.

---

## 18. COMMANDS REFERENCE

### Development
```bash
# Setup
cd rust && cargo build --target wasm32-unknown-unknown  # Verify Rust compiles
wasm-pack build --target web                            # Build WASM
cd web && npm install && npm run dev                     # Start dev server

# Test
cd rust && cargo test                                    # Rust unit/integration tests
cd rust && cargo clippy -- -D warnings                  # Lint
cd rust && cargo fmt --check                            # Format check

# Build production
wasm-pack build --target web --release                  # Optimized WASM
cd web && npm run build                                 # Production frontend

# Deploy
npx wrangler pages deploy web/dist                      # Deploy to Cloudflare
```

### Key Cargo.toml Dependencies
```toml
[package]
name = "gerberview-wasm"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
gerber_parser = "0.5"
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["console"] }
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde-wasm-bindgen = "0.6"

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "s"        # Optimize for size
lto = true             # Link-time optimization
strip = true           # Strip debug symbols
```

---

## 19. SUCCESS CRITERIA

The project is DONE when:
1. ✅ User can drop a KiCad-exported .zip and see the board rendered correctly
2. ✅ User can drop an Eagle-exported .zip and see the board rendered correctly
3. ✅ All layer types are identified and color-coded
4. ✅ Zoom and pan work smoothly at 60fps
5. ✅ Total load time < 2 seconds for a 6-layer board
6. ✅ WASM binary < 500KB gzipped
7. ✅ Works in Chrome, Firefox, Safari, Edge
8. ✅ Deployed and accessible at public URL
9. ✅ README has screenshots, architecture diagram, usage instructions
10. ✅ All CI checks pass (lint, format, test, build)

---

## 20. WHAT THE NEXT AGENT SHOULD DO FIRST

1. **Read this document completely.**
2. **Download Microsoft Pragmatic Rust Guidelines** from `https://microsoft.github.io/rust-guidelines/agents/all.txt` and load into your context.
3. **Verify gerber_parser compiles to WASM** — this is the single biggest risk. If it doesn't, research why and fix (likely need `--cfg` flags or a fork).
4. **Download test Gerber files** — Arduino Uno and KiCad sample project.
5. **Start Phase 0** — scaffold everything, get the toolchain working end-to-end (Rust → WASM → browser loads module).
6. **Then proceed phase by phase,** committing at each phase boundary.

The human will review visual output and make architectural decisions when you're stuck. Everything else is yours.
