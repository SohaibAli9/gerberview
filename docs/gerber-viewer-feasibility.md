# GerberView — End-to-End Feasibility Analysis

> **Purpose:** Resolves every caveat, risk, and assumption in the [agent brief](./gerber-viewer-agent-brief.md). Each section produces a concrete verdict (GO / GO WITH CHANGES / BLOCKED) and, where needed, a corrected approach. This document is the engineering pre-flight check before any code is written.
>
> **Date:** 2026-02-21

---

## Executive Summary

| Area | Verdict | Key Finding |
|------|---------|-------------|
| gerber_parser WASM compat | **GO** (high confidence) | API is generic over `Read`; all deps are pure Rust. No `std::fs`. Must verify with a build test. |
| WASM binary size (<500KB gz) | **GO WITH CHANGES** | `regex` adds 200-600KB to WASM. Drop `serde_json`, use raw typed arrays for vertex data. Realistic target: **300-600KB gz** after `wasm-opt -Oz`. May reach 500KB; 800KB is acceptable fallback. |
| Geometry pipeline | **GO** | `earclip` crate (v1.8.0) is WASM-ready and handles degenerate polygons. Stroke widening and arc tessellation are standard vector math. |
| Excellon parsing | **GO** | No Rust crate exists. Must write from scratch (~200-400 LOC). Format is simple. |
| Vite + wasm-pack | **GO** | `vite-plugin-wasm-pack` provides turnkey integration. |
| WebGL 1.0 | **GO** | `OES_element_index_uint` is universally supported. |
| Layer identification | **GO WITH CHANGES** | Move from Rust to TypeScript. It's pure string matching that runs before WASM is invoked. |
| Overall | **GO** | No blockers found. Two corrections to the brief, several optimizations identified. |

---

## 1. gerber_parser WASM Compatibility

### What the brief claims
> gerber_parser crate exists and is mature. Compiles to WASM. Memory-safe.
> Risk: HIGH — untested.

### What we found

**API signature** (from docs.rs):
```rust
pub fn parse<T: Read>(reader: BufReader<T>) -> Result<GerberDoc, (GerberDoc, ParseError)>
```

This is generic over any `T: Read`. In WASM, we use `std::io::Cursor<&[u8]>` which implements `Read`:
```rust
use std::io::{BufReader, Cursor};
let doc = gerber_parser::parse(BufReader::new(Cursor::new(bytes)));
```

No filesystem access required.

**Dependency audit** (from Cargo.toml on GitHub, v0.4.0):

| Dependency | WASM-safe? | Notes |
|-----------|-----------|-------|
| `gerber-types` 0.7.0 | Yes | Pure type definitions |
| `regex` 1.11.1 | Yes | Compiles to WASM. Adds 200-600KB to binary. |
| `lazy-regex` 3.4.1 | Yes | Compile-time regex validation via proc macros |
| `anyhow` 1.0.98 | Yes | Pure Rust error handling |
| `thiserror` 2.0.12 | Yes | Derive macros, no runtime deps |
| `strum` 0.27.1 | Yes | Enum derive macros |
| `log` 0.4.27 | Yes | Logging facade, no I/O |
| `env_logger` 0.11.8 | N/A | **Optional**, gated behind feature flag. Do not enable. |

No dependency uses `std::fs`, `std::net`, `std::thread`, or any OS-specific API.

**`GerberDoc` output struct:**
```rust
pub struct GerberDoc {
    pub units: Option<Unit>,
    pub format_specification: Option<CoordinateFormat>,
    pub apertures: HashMap<i32, Aperture>,
    pub commands: Vec<Result<Command, GerberParserErrorWithContext>>,
    pub image_name: Option<String>,
}
```

`HashMap` works in WASM (`std::collections` is available). The `Command` enum from `gerber-types` is the AST we walk to generate geometry.

**Partial parsing:** The return type `Result<GerberDoc, (GerberDoc, ParseError)>` means on fatal error we still get a partial `GerberDoc`. This directly supports requirement NF11 (graceful degradation on malformed files).

### Corrections to the brief

| Brief says | Reality | Impact |
|-----------|---------|--------|
| gerber_parser v0.5.0 | Latest is **v0.4.0** (Dec 19, 2025) | Use `gerber_parser = "0.4"` in Cargo.toml |
| "pure Rust" | Confirmed pure Rust, but not CI-tested against WASM | First build task must verify |

### Verdict: **GO**

Confidence: ~90%. The 10% residual risk is "something in `regex` or `gerber-types` does something unexpected on `wasm32-unknown-unknown`." Mitigated by making WASM compilation the literal first task in Phase 0.

### Mandatory first action
```bash
cargo build --target wasm32-unknown-unknown
```
If this fails, the fix is almost certainly a feature flag or a thin wrapper. Forking is unlikely to be necessary.

---

## 2. WASM Binary Size

### What the brief claims
> NF3: WASM binary size (gzipped) < 500KB

### Analysis

**Major contributors to WASM binary size:**

| Component | Estimated WASM contribution (uncompressed) | Gzipped |
|-----------|---------------------------------------------|---------|
| `regex` engine | 200-600KB | 80-250KB |
| `serde` + `serde_json` | 50-150KB | 20-60KB |
| `gerber_parser` + `gerber-types` | 30-80KB | 10-30KB |
| Our geometry code | 20-50KB | 8-20KB |
| `earclip` triangulation | 10-30KB | 4-12KB |
| wasm-bindgen glue | 10-20KB | 4-8KB |
| **Total (pessimistic)** | **~930KB** | **~380KB** |
| **Total (optimistic)** | **~320KB** | **~126KB** |

### Optimization strategy

1. **Drop `serde_json` entirely.** Vertex buffers (the bulk of data) should cross the WASM boundary as raw `Float32Array` views into WASM linear memory — zero-copy, zero serialization. Metadata (bounds, layer info) uses `serde-wasm-bindgen` which avoids JSON string allocation.

2. **Compiler flags** (already in brief, but correct `opt-level`):
   ```toml
   [profile.release]
   opt-level = "z"       # Optimize for size (not "s")
   lto = true
   codegen-units = 1     # Better optimization, slower compile
   strip = true
   ```

3. **Post-processing:** Run `wasm-opt -Oz` on the output (wasm-pack does this automatically in `--release` mode).

4. **Avoid `regex` bloat amplification:** Don't add our own regex usage. The `regex` cost comes from `gerber_parser`; we inherit it but don't amplify it.

### Revised target

| Scenario | Estimated gzipped size | Acceptable? |
|----------|----------------------|-------------|
| Best case (all optimizations) | ~150-300KB | Well within target |
| Likely case | ~300-500KB | Meets target |
| Worst case (regex bloats) | ~500-800KB | Acceptable for a portfolio project |

### Verdict: **GO WITH CHANGES**

Drop `serde_json` from dependencies. Use `opt-level = "z"` (not `"s"`). The 500KB target is achievable; if `regex` pushes us to 600-800KB, it's still acceptable (tracespace's JS bundle was ~1.2MB).

---

## 3. Geometry Pipeline

### What the brief claims
> This is the hardest part of the project. The agent MUST understand these conversions.

### Feasibility of each component

#### 3.1 Triangulation (Region Fill)

**Brief says:** Ear-clipping algorithm, possibly implemented from scratch.

**Better approach:** Use the `earclip` crate (v1.8.0, MIT license, Dec 2025):
- Explicitly WASM-compatible (published on both crates.io and npm)
- Handles: 2D/3D polygons, holes, twisted polygons, degeneracies, self-intersections
- `#![forbid(unsafe_code)]`
- 3.18KB minified/gzipped (negligible size impact)
- Uses modified ear-slicing optimized by z-order curve hashing

This eliminates the MEDIUM risk of "ear-clipping triangulation correctness" identified in Section 13 of the brief. The `earclip` crate handles exactly the degenerate cases that concerned the brief.

**Add to Cargo.toml:**
```toml
earclip = "1.8"
```

#### 3.2 Stroke Widening (D01 draws)

Standard 2D vector math:
1. Direction vector: `d = normalize(end - start)`
2. Perpendicular: `n = (-d.y, d.x)`
3. Offset: `half_width = aperture_diameter / 2`
4. Four corners: `start ± n * half_width`, `end ± n * half_width`
5. Two triangles from the quad
6. Semicircle endcaps: N-gon halves at each end (for circular apertures)

No library needed. ~50-80 LOC.

#### 3.3 Arc Tessellation (G02/G03)

Standard trigonometric tessellation:
1. Compute center from current position + I,J offset
2. `start_angle = atan2(start.y - center.y, start.x - center.x)`
3. `end_angle = atan2(end.y - center.y, end.x - center.x)`
4. Determine sweep direction (CW for G02, CCW for G03)
5. Generate N points: `p_i = center + radius * (cos(angle_i), sin(angle_i))`
6. N chosen by: `max(16, arc_length / (aperture_width * 0.25))` — adaptive tessellation
7. Each segment becomes a stroke-widened quad (reuse 3.2)

~80-120 LOC. The brief's concern about "arc math" is valid — edge cases include:
- Zero-length arcs (degenerate, skip)
- Full-circle arcs (start == end with nonzero I,J)
- Multi-quadrant mode (G75) vs. single-quadrant (G74)

**Mitigation:** Start with G75 (multi-quadrant) only. G74 is deprecated since 2021 per the Gerber spec. Log a warning if G74 is encountered.

#### 3.4 Aperture Expansion (D03 flash)

| Aperture Type | Vertex Generation | Complexity |
|--------------|-------------------|-----------|
| Circle (C) | N-gon with N=32 segments | Trivial (~20 LOC) |
| Rectangle (R) | 4 vertices, 2 triangles | Trivial (~10 LOC) |
| Obround (O) | Rectangle body + 2 semicircle endcaps | Moderate (~40 LOC) |
| Polygon (P) | Regular N-gon with rotation | Simple (~25 LOC) |

#### 3.5 Aperture Macros (AM)

The most complex aperture type. Primitives include:
- Circle (code 1)
- Vector Line (code 20)
- Center Line (code 21)
- Outline (code 4)
- Polygon (code 5)
- Moiré (code 6) — deprecated
- Thermal (code 7) — **not supported by gerber_parser**

Each primitive has an exposure flag (on/off) that adds or clears geometry. Arithmetic expressions in parameters must be evaluated.

**Approach:** Implement primitive-by-primitive. `gerber_parser` already parses the macro definitions into an AST — we just need to evaluate them into vertices. Thermal primitive (code 7) is unsupported by the parser anyway, so we skip it.

Estimated effort: ~200-300 LOC for the macro evaluator.

#### 3.6 Polarity (LPD/LPC)

**Brief recommends Option B for MVP:** Render clear polarity shapes in background color.

This is correct for MVP. Option A (stencil buffer) is a Phase 6+ enhancement. Option B works correctly as long as layers don't overlap with complex clear/dark interleaving within a single layer — which is rare in practice.

#### 3.7 Step-Repeat (SR)

Pure vertex buffer duplication with X/Y offsets. ~30-40 LOC.

### Verdict: **GO**

The geometry pipeline is the most code-intensive part (~600-800 LOC in Rust) but every component is well-understood math. The `earclip` crate eliminates the hardest algorithmic risk (triangulation). No novel algorithms are needed.

---

## 4. Excellon Drill Parser

### What the brief claims
> Implement Excellon drill parser (simpler — tool definitions + coordinates)

### Analysis

No Rust crate exists for Excellon parsing. The format is a subset of NC drill:

```
M48              ; Header start
T1C0.8           ; Tool 1, diameter 0.8mm
T2C1.0           ; Tool 2, diameter 1.0mm
%                ; Header end
T1               ; Select tool 1
X01500Y01000     ; Drill at (15.0, 10.0) in 2.4 format
X02500Y02000
T2               ; Select tool 2
X03000Y03000
M30              ; End of file
```

**Key parsing decisions:**
- Coordinate format: Usually 2.4 (2 integer, 4 decimal) in metric, 2.4 in imperial. Can also be specified in header with `METRIC` / `INCH` commands.
- Leading/trailing zero suppression: `TZ` (trailing zeros) or `LZ` (leading zeros)
- Tool definitions: `T<num>C<diameter>` or `T<num>` with diameter in header

**Output:** List of `DrillHole { x: f64, y: f64, diameter: f64 }` — rendered as filled circles (N-gon flash at each position).

**Estimated effort:** 200-400 LOC. Reference implementations exist in Python (pcb-tools) and C++ (KiCad's `excellon_read_drill_file.cpp`).

### Verdict: **GO**

Simple format. The brief's scope is correct (drill holes only, no routing).

---

## 5. WASM-to-JS Data Transfer

### What the brief claims
> Return Float32Array (positions) + metadata (bounds, layer info)

### Optimal approach (correcting brief)

The brief includes `serde_json` in Cargo.toml. This is unnecessary and costly for binary size.

**Vertex data (hot path):** Zero-copy `Float32Array` view into WASM linear memory:

```rust
use js_sys::Float32Array;
use wasm_bindgen::memory;

pub fn get_vertex_buffer(positions: &[f32]) -> Float32Array {
    let memory = wasm_bindgen::memory()
        .dyn_into::<js_sys::WebAssembly::Memory>().unwrap()
        .buffer();
    let offset = positions.as_ptr() as u32 / 4;
    Float32Array::new(&memory)
        .subarray(offset, offset + positions.len() as u32)
}
```

**Caveat:** The WASM memory buffer can be invalidated by any allocation. The JS side must **immediately copy** the returned `Float32Array` into a new buffer or upload to a WebGL VBO before calling any other WASM function.

**Metadata (cold path):** Use `serde-wasm-bindgen` to pass structs directly as JS objects without JSON string intermediary:

```rust
use serde::Serialize;
use serde_wasm_bindgen::to_value;

#[derive(Serialize)]
pub struct LayerMeta {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
    pub vertex_count: u32,
    pub command_count: u32,
}
```

**Index data:** Same zero-copy approach as vertex data, using `Uint32Array`.

### Revised dependencies

```toml
# REMOVE these from the brief's Cargo.toml:
# serde_json = "1"           # Not needed - saves ~50-150KB WASM

# KEEP:
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
```

### Verdict: **GO WITH CHANGES**

Remove `serde_json`. Use typed array views for vertex/index data, `serde-wasm-bindgen` for metadata.

---

## 6. Vite + wasm-pack Integration

### What the brief claims
> Vite or similar bundler (web/vite.config.ts)

### Validated approach

`vite-plugin-wasm-pack` provides direct integration:

```ts
// vite.config.ts
import { defineConfig } from 'vite';
import wasmPack from 'vite-plugin-wasm-pack';

export default defineConfig({
  plugins: [wasmPack('../rust')]
});
```

**Dev workflow:**
```json
{
  "scripts": {
    "wasm:build": "wasm-pack build ../rust --target web",
    "dev": "npm run wasm:build && vite",
    "build": "npm run wasm:build && vite build"
  }
}
```

**Auto-rebuild on Rust changes:** Use `vite-plugin-wasm-pack-watcher` (optional, nice-to-have for development).

### Known friction points

1. **wasm-pack `--target web` vs `--target bundler`:** Use `--target web` for Vite. The `bundler` target assumes webpack-style WASM loading.
2. **Top-level await:** WASM module initialization is async. Modern browsers support top-level await; for older browsers, add `vite-plugin-top-level-await`.
3. **Production build path:** WASM file must be in the final `dist/` output. Vite handles this automatically when using the plugin.

### Verdict: **GO**

Well-supported, multiple plugins available, documented workflow.

---

## 7. WebGL Rendering

### What the brief claims
> WebGL 1.0 baseline, 60fps target, dark background, per-layer color + alpha

### Validation

**`OES_element_index_uint`:** Required for `gl.UNSIGNED_INT` in `drawElements()`. Supported in Chrome 24+, Firefox 24+, Safari 8+, Edge 12+. Universal — not a concern.

**`OES_vertex_array_object` (VAO):** Not mentioned in the brief. WebGL 1.0 doesn't have native VAOs. Two options:
- Use the `OES_vertex_array_object` extension (widely supported)
- Manage VBO bindings manually per draw call (simpler for MVP)

**Recommendation:** Manual VBO management for MVP. VAOs are a micro-optimization that doesn't matter with <20 draw calls per frame.

**Draw call count:** One per visible layer, typically 7-12 layers. Well within WebGL budget.

**Vertex count budget:** A complex 6-layer board might produce ~100K-500K vertices. WebGL handles millions of vertices at 60fps on modern GPUs. Not a concern.

**Depth/ordering:** Layers are rendered back-to-front with alpha blending. No depth buffer needed. The brief's pseudocode is correct:
```
gl.enable(gl.BLEND)
gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA)
```

### One correction to the brief's shader code

The vertex shader uses a `mat3` uniform but `gl_Position` requires `vec4`. The brief handles this correctly with `vec4(pos.xy, 0.0, 1.0)`. However, `uniformMatrix3fv` is correct for a 3x3 matrix — no issue here.

### Verdict: **GO**

The WebGL approach is sound. No changes needed.

---

## 8. Layer Identification

### What the brief claims
> Implemented in `rust/src/layer_id.rs`

### Better approach

Layer identification is pure string pattern matching on filenames. It runs **before** any WASM is invoked (the JS side needs to know the layer type to decide which parser to call and what color to assign). Putting it in Rust means:
- Extra WASM roundtrip for no computational benefit
- Harder to iterate on (Rust rebuild required for pattern changes)
- `whats-that-gerber` npm package already implements this in JS

**Recommendation:** Implement in TypeScript, not Rust. Port the logic from `whats-that-gerber` (or use it directly as a dependency, though it's small enough to inline).

Additionally, the brief only lists KiCad, Eagle, and Altium patterns. EasyEDA/LCEDA patterns should be added:

| Pattern | Layer |
|---------|-------|
| `*.GTL` or `*-F_Cu*` | Top copper |
| `*.GBL` or `*-B_Cu*` | Bottom copper |
| `Gerber_TopLayer.GTL` | Top copper (EasyEDA) |
| `Gerber_BottomLayer.GBL` | Bottom copper (EasyEDA) |
| etc. | |

**Fallback strategy** (as the brief mentions): If filename matching fails, inspect file content for `%FSLAX*%` (Gerber header) or `M48` (Excellon header). Assign as "unknown layer" with a default color and let the user re-label.

### Verdict: **GO WITH CHANGES**

Move to TypeScript. Add EasyEDA patterns. Keep content-based fallback.

---

## 9. Offline / Service Worker Support

### What the brief claims
> NF5: Works offline after first load. Service worker + cache.

### Feasibility

All assets are static (HTML, CSS, JS, WASM). A service worker with a cache-first strategy works perfectly:

```js
// sw.js — cache-first for all assets
self.addEventListener('install', e => {
  e.waitUntil(
    caches.open('gerberview-v1').then(cache =>
      cache.addAll(['/', '/index.html', '/gerberview_wasm_bg.wasm', ...])
    )
  );
});
```

**WASM caching:** The `.wasm` file is fetched and cached like any other static asset. Browsers that support `WebAssembly.compileStreaming` will compile the WASM while downloading, and the compiled module can be stored in `Cache API` for instant startup on subsequent visits.

**No-backend constraint:** Fully satisfied. No API calls, no telemetry, no analytics — pure static site.

### Verdict: **GO**

Standard PWA pattern. No complications.

---

## 10. Performance Targets

### NF1: Parse + render < 2 seconds

**Parsing:** `gerber_parser` runs in Rust/WASM at near-native speed. A typical 6-layer board with ~20K commands per layer should parse in 50-200ms.

**Geometry conversion:** CPU-bound vertex generation. Estimated 100-500ms for a complex board depending on arc density and region complexity.

**WebGL upload + first render:** VBO upload for ~500K vertices: <10ms. First draw call: <1ms.

**Total estimate:** 200-700ms for a typical board. Well within the 2-second target.

**Risk:** Very complex boards (dense ground planes with many regions) could push geometry conversion to 1-2 seconds. Mitigated by:
- Using Web Workers for parsing/geometry so the UI remains responsive
- Showing a progress indicator per layer

### NF2: 60fps during interaction

During interaction, no parsing or geometry conversion occurs. The render loop is:
1. Update 3x3 view matrix (1 multiply)
2. Set uniform
3. 7-12 draw calls

This is trivially 60fps on any GPU from the last decade.

### Verdict: **GO**

Performance targets are conservative and achievable.

---

## 11. Project Structure Corrections

### Revised Cargo.toml

```toml
[package]
name = "gerberview-wasm"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
gerber_parser = "0.4"              # Corrected from "0.5"
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["console"] }
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"
earclip = "1.8"                    # Added: WASM-ready triangulation
# serde_json REMOVED — use serde-wasm-bindgen instead

[dev-dependencies]
wasm-bindgen-test = "0.3"

[profile.release]
opt-level = "z"                    # Changed from "s" to "z" for smaller binary
lto = true
codegen-units = 1                  # Added: better optimization
strip = true
```

### Revised project structure

```
gerberview/
├── rust/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                 # WASM entry points
│   │   ├── geometry/
│   │   │   ├── mod.rs
│   │   │   ├── aperture.rs        # Aperture → shape conversion
│   │   │   ├── stroke.rs          # D01 draw → stroke-expanded geometry
│   │   │   ├── arc.rs             # Arc interpolation + tessellation
│   │   │   ├── region.rs          # Region fill (delegates to earclip)
│   │   │   ├── polarity.rs        # LPD/LPC handling
│   │   │   ├── macro_eval.rs      # Aperture macro primitive evaluation
│   │   │   └── types.rs           # VertexBuffer, BoundingBox, LayerGeometry
│   │   └── excellon/
│   │       ├── mod.rs
│   │       └── parser.rs          # Excellon drill file parser
│   └── tests/
│       ├── ...
│       └── fixtures/
├── web/
│   ├── index.html
│   ├── src/
│   │   ├── main.ts
│   │   ├── viewer.ts              # WebGL rendering
│   │   ├── shaders/
│   │   │   ├── vertex.glsl
│   │   │   └── fragment.glsl
│   │   ├── interaction.ts         # Zoom/pan/touch
│   │   ├── ui.ts                  # Layer panel, upload zone
│   │   ├── zip-handler.ts         # ZIP extraction + file routing
│   │   └── layer-identify.ts      # MOVED FROM RUST: filename → layer type
│   ├── package.json
│   └── vite.config.ts
├── ...
```

**Changes from brief:**
- Removed `rust/src/layer_id.rs` → moved to `web/src/layer-identify.ts`
- Added `rust/src/geometry/polarity.rs` (was implicit)
- Added `rust/src/geometry/macro_eval.rs` (was implicit)
- Removed `rust/src/geometry/triangulate.rs` (replaced by `earclip` crate)

---

## 12. Risk Register (Updated)

| # | Risk | Severity | Status | Mitigation |
|---|------|----------|--------|-----------|
| R1 | gerber_parser won't compile to WASM | HIGH | **90% resolved** — deps audited, no blockers found | Verify with `cargo build --target wasm32-unknown-unknown` as first action |
| R2 | WASM binary exceeds 500KB gzipped | MEDIUM | **Mitigated** — removed serde_json, added size optimizations | Accept up to 800KB. Profile and strip if needed. |
| R3 | Triangulation fails on degenerate polygons | MEDIUM | **Resolved** — `earclip` handles degeneracies | Use earclip crate, add error fallback for extreme cases |
| R4 | Arc math edge cases | MEDIUM | **Mitigated** — implement G75 only, skip G74 | Log warning on G74, test against real files |
| R5 | WASM memory invalidation on JS side | MEDIUM | **New risk** — Float32Array view into WASM memory can be invalidated by Rust allocation | Copy or upload to VBO immediately after receiving from WASM |
| R6 | Touch interaction quality | LOW | Unchanged | Basic implementation, iterate based on testing |
| R7 | Large board memory usage | LOW | Unchanged | Monitor vertex counts, implement LOD if >1M vertices |
| R8 | gerber_parser missing features (Thermal macro, G74 arcs) | LOW | **Accepted** | Modern CAD output doesn't use these deprecated features |

---

## 13. Caveats Resolved

### Caveat 1: "gerber_parser WASM compatibility — untested"
**Resolution:** Dependency audit shows all deps are pure Rust with no OS-specific code. The `parse()` function is generic over `Read`, accepting `Cursor<&[u8]>` in WASM. The `env_logger` dependency is behind an optional feature flag and won't be compiled. Confidence: high. Verify with a build test.

### Caveat 2: "Ear-clipping triangulation correctness"
**Resolution:** Use `earclip` v1.8.0 instead of a custom implementation. It handles self-intersecting polygons, holes, and degeneracies. It's WASM-tested and MIT-licensed.

### Caveat 3: "WASM memory limits — large boards"
**Resolution:** A very complex board (1M vertices × 2 floats × 4 bytes = 8MB) is well within WASM's default memory limit. The concern is valid only for panelized boards with hundreds of instances — explicitly out of MVP scope.

### Caveat 4: "WebGL 1.0 vs 2.0"
**Resolution:** WebGL 1.0 with `OES_element_index_uint` extension (universally supported) is sufficient. No WebGL 2.0 features are needed for MVP.

### Caveat 5: "Agent may forget WASM memory patterns"
**Resolution:** The `Float32Array` view approach is documented above with the critical caveat about memory invalidation. The WASM→JS contract: "copy the typed array before calling another WASM export."

### Caveat 6: "gerber_parser version discrepancy"
**Resolution:** Brief says v0.5.0, actual latest is v0.4.0 (Dec 2025). Use `gerber_parser = "0.4"`. No functional impact — v0.4.0 has full Gerber 2024.05 spec compliance.

### Caveat 7: "serde_json binary size cost"
**Resolution:** Remove `serde_json` entirely. Use zero-copy `Float32Array` for vertex data and `serde-wasm-bindgen` for metadata. This saves an estimated 50-150KB of WASM binary size.

### Caveat 8: "Layer identification in Rust"
**Resolution:** Move to TypeScript. It's pure string matching, runs before WASM, and the `whats-that-gerber` npm package provides reference patterns. No reason to pay the Rust-rebuild tax for pattern updates.

---

## 14. Go/No-Go Checklist

| # | Question | Answer |
|---|----------|--------|
| 1 | Does the core parser exist and compile to our target? | Yes (high confidence, verify on first build) |
| 2 | Is the geometry math well-defined? | Yes — standard 2D vector/trig, no novel algorithms |
| 3 | Does a triangulation solution exist? | Yes — `earclip` crate, WASM-ready |
| 4 | Is the rendering approach proven? | Yes — WebGL flat 2D triangles with alpha blending |
| 5 | Can we transfer data WASM→JS efficiently? | Yes — zero-copy Float32Array views |
| 6 | Does the build toolchain work? | Yes — wasm-pack + Vite, well-documented |
| 7 | Is the hosting solution viable? | Yes — Cloudflare Pages, free, static files |
| 8 | Are test fixtures available? | Yes — Arduino Uno, KiCad samples, tracespace fixtures |
| 9 | Are there any legal/licensing blockers? | No — all deps are MIT or Apache-2.0 |
| 10 | Is the scope achievable? | Yes — 5 phases, each independently testable |

---

## 15. Recommended Execution Order (Revised)

The brief's phased plan is sound. These are the adjustments:

### Phase 0 adjustments
- **Add:** `cargo build --target wasm32-unknown-unknown` with `gerber_parser` dependency as the absolute first task. If this fails, everything else is blocked.
- **Add:** Install `earclip` and verify it compiles to WASM alongside `gerber_parser`.
- **Change:** Layer identification lives in `web/src/layer-identify.ts`, not Rust.

### Phase 1 adjustments
- **Change:** `parse_gerber()` returns metadata via `serde-wasm-bindgen`, not JSON.
- **Add:** Verify the `Command` enum from `gerber-types` covers all needed Gerber commands by parsing a real file and logging all command variants.

### Phase 2 adjustments
- **Change:** Use `earclip` for triangulation instead of custom ear-clipping.
- **Add:** `macro_eval.rs` as explicit module for aperture macro evaluation.
- **Add:** Test with at least 3 real-world board files (KiCad, Eagle, Altium-exported).

### Phase 3 adjustments
- **Add:** Copy `Float32Array` immediately after receiving from WASM (memory invalidation guard).
- **Note:** Skip VAOs — use manual VBO binding for MVP.

### Phase 5 adjustments
- **Add:** Measure actual WASM binary size gzipped. If >800KB, profile with `twiggy` and strip unused code.

---

## 16. Final Verdict

**The project is feasible.** No blockers were found. The eight caveats from the brief have been resolved — three by changing approach (triangulation crate, layer ID in TS, drop serde_json), three by confirming the original approach works (WASM compat, WebGL, memory), and two by accepting known limitations (parser version, deprecated Gerber features).

The single remaining gating risk is the Phase 0 WASM build test for `gerber_parser`. If that succeeds (expected), the rest is execution.
