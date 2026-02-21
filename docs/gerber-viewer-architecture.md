# GerberView — Architecture & Design Document

> **Document ID:** GVARCH-001  
> **Version:** 1.0.0  
> **Date:** 2026-02-21  
> **Status:** Draft  
> **Standards Reference:** IEEE 42010:2022 (Architecture Description), ISO/IEC 25010:2023  
> **Upstream:** [Spec](./gerber-viewer-spec.md), [Feasibility](./gerber-viewer-feasibility.md), [Brief](./gerber-viewer-agent-brief.md)

---

## Table of Contents

1. [Architectural Drivers](#1-architectural-drivers)
2. [System Context](#2-system-context)
3. [High-Level Decomposition](#3-high-level-decomposition)
4. [Thread Architecture](#4-thread-architecture)
5. [Reactive State Management](#5-reactive-state-management)
6. [Scene Graph](#6-scene-graph)
7. [Web Worker Protocol](#7-web-worker-protocol)
8. [WASM Bridge Design](#8-wasm-bridge-design)
9. [Rendering Pipeline](#9-rendering-pipeline)
10. [Geometry Pipeline (Rust)](#10-geometry-pipeline-rust)
11. [Data Flow — End to End](#11-data-flow--end-to-end)
12. [Memory Management](#12-memory-management)
13. [Error Propagation Architecture](#13-error-propagation-architecture)
14. [Module Dependency Rules](#14-module-dependency-rules)
15. [Updated Project Structure](#15-updated-project-structure)
16. [Component Catalog](#16-component-catalog)
17. [Design Patterns](#17-design-patterns)
18. [Key Algorithms](#18-key-algorithms)
19. [Service Worker Architecture](#19-service-worker-architecture)
20. [Extension Points](#20-extension-points)
21. [Decision Log](#21-decision-log)

---

## 1. Architectural Drivers

These quality attributes, ranked by priority, shape every structural decision in this document. When trade-offs arise, higher-ranked attributes win.

| Rank | Attribute | Constraint | Governing Requirement |
|------|-----------|-----------|----------------------|
| 1 | **Privacy** | Zero network I/O after initial load | NFR-300 |
| 2 | **Responsiveness** | UI never blocks >50ms during parsing | NFR-100, FR-605 |
| 3 | **Correctness** | Geometry output matches the Gerber spec | FR-300–FR-307 |
| 4 | **Render performance** | 60fps during interaction | NFR-101 |
| 5 | **Bundle size** | <800KB WASM gzipped, <1.5MB total | NFR-103, NFR-104 |
| 6 | **Extensibility** | Adding overlays/measurements without renderer rewrite | Decision: scene graph |
| 7 | **Testability** | 90% coverage, E2E with Playwright | NFR-500, NFR-501 |
| 8 | **Accessibility** | WCAG 2.1 AA | Section 18 of spec |

### Architectural Non-Goals

- Server-side rendering (no SSR, no SSG with dynamic content)
- Framework adoption (no React, Vue, Svelte — vanilla TS is sufficient)
- Real-time collaboration
- Plugin system / third-party extensions

---

## 2. System Context

```
┌──────────────────────────────────────────────────────────────────┐
│                        USER'S BROWSER                            │
│                                                                  │
│   ┌──────────────────────────────────────────────────────────┐   │
│   │              GerberView Application                      │   │
│   │                                                          │   │
│   │   User's filesystem ──(File API)──► Application          │   │
│   │                                                          │   │
│   │   Application ──(WebGL)──► GPU                           │   │
│   │                                                          │   │
│   │   Application ──(Service Worker)──► Cache API            │   │
│   │                                                          │   │
│   └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│   External boundary:                                             │
│     • No network I/O after initial page load                     │
│     • No cookies, localStorage, or IndexedDB                     │
│     • No third-party scripts or iframes                          │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘

        ▲ Initial load only (HTTPS)
        │
┌───────┴──────────┐
│ Cloudflare Pages  │   Static assets: HTML, CSS, JS, WASM
│ (CDN)            │   No dynamic backend
└──────────────────┘
```

**Trust boundary:** The only external input is the user's ZIP file. All other code is first-party static assets loaded over HTTPS.

---

## 3. High-Level Decomposition

The application is split into four architectural layers. Each layer has a strict dependency direction: layers MAY depend on layers below them, MUST NOT depend on layers above.

```
┌─────────────────────────────────────────────────────────────────┐
│                       PRESENTATION LAYER                         │
│                                                                  │
│   UI Module          Interaction Module       Upload Module      │
│   (ui.ts)            (interaction.ts)         (zip-handler.ts)   │
│                                                                  │
│   Depends on: State, Scene                                       │
├─────────────────────────────────────────────────────────────────┤
│                        STATE LAYER                               │
│                                                                  │
│   Reactive Store     Signals                                     │
│   (store.ts)         (signal.ts)                                 │
│                                                                  │
│   Depends on: Types only                                         │
├─────────────────────────────────────────────────────────────────┤
│                        SCENE LAYER                               │
│                                                                  │
│   Scene Graph        Renderer                                    │
│   (scene.ts)         (renderer.ts)                               │
│                                                                  │
│   Depends on: State (reads signals), WebGL                       │
├─────────────────────────────────────────────────────────────────┤
│                        ENGINE LAYER                              │
│                                                                  │
│   Parse Worker       WASM Bridge          Layer Identify         │
│   (parse-worker.ts)  (wasm-bridge.ts)     (layer-identify.ts)    │
│                                                                  │
│   Depends on: WASM module, Types                                 │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────────────────────────────┐
│                     RUST / WASM LAYER                            │
│                                                                  │
│   lib.rs → geometry/ → excellon/                                 │
│                                                                  │
│   Runs inside Web Worker. No DOM access.                         │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Thread Architecture

Parsing and geometry conversion run on a dedicated Web Worker. The main thread handles UI, rendering, and user interaction. This guarantees the UI remains responsive during heavy computation.

```
┌─ Main Thread ───────────────────────────┐    ┌─ Parse Worker ──────────────┐
│                                          │    │                             │
│  User Input ──► Interaction ──► State    │    │  WASM Module (initialized   │
│                                 │        │    │  once on worker start)      │
│  State ──► Scene Graph ──► Renderer      │    │                             │
│                    │                     │    │  Receives: file bytes       │
│  UI ◄── State (subscriptions)            │    │  Calls:    parse_gerber()   │
│                    │                     │    │            get_positions()   │
│  Upload ──► [postMessage] ──────────────►│────│► Runs:     geometry pipeline│
│             [Transferable]               │    │                             │
│                    ◄─────────────────────│◄───│◄ Returns:  buffers + meta   │
│  State update ◄── [onmessage]            │    │            [Transferable]   │
│       │                                  │    │                             │
│       ▼                                  │    └─────────────────────────────┘
│  Scene Graph updated                     │
│  Renderer dirty flag set                 │
│  requestAnimationFrame → draw            │
│                                          │
└──────────────────────────────────────────┘
```

### 4.1 Thread Ownership Rules

| Resource | Owner | Access by other thread |
|----------|-------|----------------------|
| DOM | Main thread | Never (Worker has no DOM) |
| WebGL context | Main thread | Never |
| WASM module instance | Worker thread | Never (separate instance) |
| Reactive store | Main thread | Never |
| Scene graph | Main thread | Never |
| `Float32Array` vertex buffers | Transferred from Worker → Main | Transferred via `postMessage`, zero-copy |
| `Uint32Array` index buffers | Transferred from Worker → Main | Transferred via `postMessage`, zero-copy |

### 4.2 Worker Lifecycle

```
Main thread                              Worker thread
    │                                         │
    │──── new Worker("parse-worker.ts") ─────►│
    │                                         │
    │◄─── { type: "ready" } ─────────────────│  (WASM module initialized)
    │                                         │
    │  ... user drops a file ...              │
    │                                         │
    │──── { type: "parse",                    │
    │       files: IdentifiedFile[] } ───────►│
    │       (bytes Transferred)               │
    │                                         │──── parse_gerber(bytes)
    │                                         │──── get_positions() → copy
    │                                         │──── get_indices() → copy
    │                                         │
    │◄─── { type: "layer-result",             │
    │       id, meta,                         │
    │       positions: Float32Array,          │
    │       indices: Uint32Array } ──────────│
    │       (buffers Transferred)             │
    │                                         │
    │  ... repeat for each file ...           │
    │                                         │
    │◄─── { type: "complete",                 │
    │       totalLayers, totalWarnings } ────│
    │                                         │
```

### 4.3 Why Not SharedArrayBuffer

`SharedArrayBuffer` requires `Cross-Origin-Isolation` headers (`COOP` and `COEP`), which prevent loading third-party resources and complicate Cloudflare Pages configuration. Since our data transfer is one-shot per layer (not streaming), `Transferable` ownership transfer is simpler, zero-copy, and has no header requirements.

---

## 5. Reactive State Management

The application uses a custom lightweight signals/reactive-store pattern. No framework dependency. Signals are the single source of truth for all application state.

### 5.1 Signal Primitive

```typescript
/** A reactive container for a value. Subscribers are notified on change. */
export interface ReadonlySignal<T> {
  readonly value: T;
  subscribe(fn: (value: T) => void): () => void;
}

export interface Signal<T> extends ReadonlySignal<T> {
  value: T;
  update(fn: (current: T) => T): void;
}
```

Implementation: ~40 LOC. Each `Signal<T>` holds a `Set<Subscriber>`. Setting `.value` or calling `.update()` synchronously notifies all subscribers. `subscribe()` returns an unsubscribe function.

### 5.2 Computed Signal

```typescript
/** A signal derived from other signals. Recomputes when dependencies change. */
export interface Computed<T> extends ReadonlySignal<T> {}
```

Implementation: subscribes to source signals, recomputes on change, notifies own subscribers. Lazy evaluation: only recomputes if read or if it has subscribers.

### 5.3 Application Store

```typescript
/** All application state lives in this single reactive store. */
export interface AppStore {
  /** Current application lifecycle state. */
  readonly appState: Signal<AppState>;

  /** All loaded layers (empty array when no file loaded). */
  readonly layers: Signal<readonly ParsedLayer[]>;

  /** Current view transform (zoom, pan). */
  readonly viewState: Signal<ViewState>;

  /** Current global opacity multiplier [0, 1]. */
  readonly globalOpacity: Signal<number>;

  /** Active error, if any. */
  readonly error: Signal<AppError | null>;

  /** Loading progress: { current: number, total: number, label: string } | null */
  readonly loadingProgress: Signal<LoadingProgress | null>;

  /** Cursor position in board coordinates (null when cursor off-canvas). */
  readonly cursorPosition: Signal<Point | null>;

  // --- Computed ---

  /** Visible layers (filtered from layers where visible === true). */
  readonly visibleLayers: Computed<readonly ParsedLayer[]>;

  /** Union bounding box of all layers. */
  readonly boardBounds: Computed<BoundingBox | null>;

  /** Board dimensions in mm (from outline layer or full bounds). */
  readonly boardDimensions: Computed<{ width: number; height: number } | null>;

  /** Count of all warnings across all layers. */
  readonly totalWarnings: Computed<number>;
}
```

### 5.4 State Flow Diagram

```
User Action              State Mutation                Side Effect
─────────────────────────────────────────────────────────────────────

Drop .zip ──────────► appState = Loading          ──► UI shows spinner
                      loadingProgress = {0, N}        Worker receives files

Worker returns ─────► layers.push(newLayer)       ──► Scene graph adds node
  layer result        loadingProgress = {i, N}        Renderer dirty = true

Worker complete ────► appState = Rendered         ──► UI shows layer panel
                      loadingProgress = null          Fit-to-view triggered

Toggle layer ───────► layers[i].visible = !v      ──► Scene node toggled
                                                      Renderer dirty = true

Scroll zoom ────────► viewState.zoom *= factor    ──► Renderer dirty = true
                      viewState.center adjusted

Click-drag pan ─────► viewState.center += delta   ──► Renderer dirty = true

Fit-to-view ────────► viewState = computed fit    ──► Renderer dirty = true

Opacity slider ─────► globalOpacity = value       ──► Renderer dirty = true

Error occurs ───────► appState = Error            ──► UI shows error banner
                      error = { code, message }
```

### 5.5 Subscription Topology

```
                    ┌──────────────────────────────────────┐
                    │            AppStore                   │
                    │                                      │
                    │  appState ──────────► UI.renderState  │
                    │  layers ────────────► UI.layerPanel   │
                    │                  └──► Scene.sync      │
                    │  viewState ─────────► Renderer.dirty  │
                    │  globalOpacity ─────► Renderer.dirty  │
                    │  error ─────────────► UI.errorBanner  │
                    │  loadingProgress ───► UI.spinner      │
                    │  cursorPosition ────► UI.coordDisplay │
                    │  boardBounds ───────► Renderer.fit    │
                    │  boardDimensions ──► UI.dimensions    │
                    │                                      │
                    └──────────────────────────────────────┘
```

Every arrow is a reactive subscription. When the signal on the left changes, the subscriber on the right runs. No polling, no manual invalidation.

---

## 6. Scene Graph

The scene graph is a thin abstraction between the parsed layer data and the WebGL renderer. It exists so that the renderer does not know about Gerber/Excellon domain concepts — it only knows about renderable nodes.

### 6.1 Node Hierarchy

```
SceneRoot
 ├── BoardNode (aggregate bounds, coordinate system)
 │    ├── LayerNode ("top_copper", visible, color, opacity, VBO handles)
 │    ├── LayerNode ("bottom_copper", ...)
 │    ├── LayerNode ("top_solder_mask", ...)
 │    ├── ...
 │    └── LayerNode ("drill", ...)
 └── OverlayGroup (future: measurement lines, cursor crosshair, grid)
      ├── CrosshairOverlay (follows cursor, different shader)
      └── (future: MeasurementOverlay, GridOverlay)
```

### 6.2 Scene Node Types

```typescript
/** Base for all scene nodes. */
export interface SceneNode {
  readonly id: string;
  visible: boolean;
}

/** A renderable layer with GPU buffer handles. */
export interface LayerNode extends SceneNode {
  readonly kind: "layer";
  readonly layerType: LayerType;
  readonly color: LayerColor;
  readonly renderState: LayerRenderState | null;
  readonly meta: LayerMeta;
  readonly zOrder: number;
  opacity: number;
}

/** The root of all board-space layers. */
export interface BoardNode extends SceneNode {
  readonly kind: "board";
  readonly layers: readonly LayerNode[];
  readonly bounds: BoundingBox;
}

/** Container for non-board overlays (cursor crosshair, future measurements). */
export interface OverlayGroup extends SceneNode {
  readonly kind: "overlay-group";
  readonly children: readonly OverlayNode[];
}

/** A single overlay element (crosshair, ruler, grid). */
export interface OverlayNode extends SceneNode {
  readonly kind: "crosshair" | "measurement" | "grid";
  readonly renderFn: (gl: WebGLRenderingContext, viewMatrix: ViewMatrix) => void;
}

/** The complete scene. */
export interface SceneRoot {
  board: BoardNode | null;
  overlays: OverlayGroup;
}
```

### 6.3 Scene Manager

```typescript
/** Manages the scene graph lifecycle: add layers, remove, sync with store. */
export class SceneManager {
  private readonly scene: SceneRoot;
  private readonly gl: WebGLRenderingContext;

  /** Add a parsed layer: upload VBOs, create LayerNode, insert in z-order. */
  addLayer(layer: ParsedLayer): void;

  /** Remove all layers and release GPU resources. */
  clear(): void;

  /** Return all visible LayerNodes sorted by z-order (back to front). */
  getVisibleLayers(): readonly LayerNode[];

  /** Return the union bounding box of all layers. */
  getBounds(): BoundingBox | null;

  /** Release all WebGL buffers. Called on context loss. */
  releaseGPUResources(): void;

  /** Re-upload all VBOs. Called on context restore. */
  restoreGPUResources(): void;
}
```

### 6.4 Z-Order (Render Order)

Layers are rendered back-to-front for correct alpha compositing:

| Z-Order | Layer Type | Rationale |
|---------|-----------|-----------|
| 0 | Board outline | Behind everything |
| 1 | Bottom paste | |
| 2 | Bottom solder mask | |
| 3 | Bottom silkscreen | |
| 4 | Bottom copper | |
| 5 | Inner copper (1..N) | |
| 6 | Top copper | |
| 7 | Top silkscreen | |
| 8 | Top solder mask | |
| 9 | Top paste | |
| 10 | Drill holes | On top of everything |
| 100+ | Overlays | Always topmost |

### 6.5 Why a Scene Graph (vs. Direct Rendering)

| Concern | Without scene graph | With scene graph |
|---------|-------------------|-----------------|
| Adding a cursor crosshair overlay | Modify renderer internals | Add OverlayNode, renderer iterates nodes generically |
| Adding measurement tool (post-MVP) | Major renderer refactor | Add MeasurementOverlay node + render function |
| WebGL context loss/restore | Renderer must track all buffers manually | SceneManager.releaseGPU/restoreGPU handles it centrally |
| Testability | Renderer tightly coupled to domain | Scene graph can be unit-tested without WebGL |
| Rendering order | Hardcoded in renderer | Z-order sort on scene nodes |

The scene graph is deliberately thin (~150 LOC for SceneManager). It does not have transform hierarchies, spatial partitioning, or entity-component patterns — those would be over-engineering for a 2D viewer.

---

## 7. Web Worker Protocol

All communication between the main thread and the parse worker uses a typed message protocol. No stringly-typed messages.

### 7.1 Message Types — Main → Worker

```typescript
/** Messages sent from main thread to worker. */
export type MainToWorkerMessage =
  | ParseRequestMessage
  | CancelMessage;

export interface ParseRequestMessage {
  readonly type: "parse-request";
  readonly requestId: string;
  readonly files: readonly FilePayload[];
}

export interface FilePayload {
  readonly fileName: string;
  readonly layerType: LayerType;
  readonly fileType: "gerber" | "excellon";
  readonly content: ArrayBuffer;      // Transferred, not copied
}

export interface CancelMessage {
  readonly type: "cancel";
  readonly requestId: string;
}
```

### 7.2 Message Types — Worker → Main

```typescript
/** Messages sent from worker to main thread. */
export type WorkerToMainMessage =
  | WorkerReadyMessage
  | LayerResultMessage
  | LayerErrorMessage
  | ParseCompleteMessage;

export interface WorkerReadyMessage {
  readonly type: "worker-ready";
}

export interface LayerResultMessage {
  readonly type: "layer-result";
  readonly requestId: string;
  readonly fileName: string;
  readonly layerType: LayerType;
  readonly meta: LayerMeta;
  readonly positions: Float32Array;   // Transferred, not copied
  readonly indices: Uint32Array;      // Transferred, not copied
}

export interface LayerErrorMessage {
  readonly type: "layer-error";
  readonly requestId: string;
  readonly fileName: string;
  readonly error: string;
}

export interface ParseCompleteMessage {
  readonly type: "parse-complete";
  readonly requestId: string;
  readonly totalLayers: number;
  readonly successCount: number;
  readonly errorCount: number;
  readonly totalWarnings: number;
  readonly elapsedMs: number;
}
```

### 7.3 Transfer Semantics

```typescript
// Main → Worker: transfer ArrayBuffer ownership
worker.postMessage(message, [file1.content, file2.content, ...]);

// Worker → Main: transfer typed array ownership
self.postMessage(result, [result.positions.buffer, result.indices.buffer]);
```

After transfer, the sender's reference becomes a zero-length detached buffer. This is intentional: it prevents accidental dual-ownership and provides zero-copy performance.

### 7.4 Request Cancellation

If the user drops a new file while a previous parse is in progress:

1. Main thread sends `{ type: "cancel", requestId: oldId }`.
2. Main thread sends `{ type: "parse-request", requestId: newId, ... }`.
3. Worker checks `requestId` before posting each result. If the ID is stale, the result is dropped.
4. Main thread ignores any messages with a stale `requestId`.

This is cooperative cancellation. The Rust WASM parse function runs synchronously per file and cannot be interrupted mid-parse. But layer-level cancellation (skip remaining files) works.

---

## 8. WASM Bridge Design

The WASM bridge is the boundary between JavaScript and Rust. It runs inside the Web Worker.

### 8.1 Rust Exports

```rust
// crates/gerberview-wasm/src/lib.rs

/// Initialize the WASM module (called once on Worker start).
/// Sets up panic hook for debugging.
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Parse a Gerber RS-274X file and generate renderable geometry.
///
/// Returns LayerMeta via serde-wasm-bindgen.
/// Stores geometry internally; retrieve with get_positions() / get_indices().
///
/// # Errors
/// Returns a descriptive error string if parsing fails fatally.
#[wasm_bindgen]
pub fn parse_gerber(data: &[u8]) -> Result<JsValue, JsValue>;

/// Parse an Excellon drill file and generate renderable geometry.
///
/// Same pattern as parse_gerber.
#[wasm_bindgen]
pub fn parse_excellon(data: &[u8]) -> Result<JsValue, JsValue>;

/// Retrieve the position buffer for the last parsed layer.
///
/// Returns a **copy** of the positions as a new Float32Array.
/// Safe to transfer to main thread.
#[wasm_bindgen]
pub fn get_positions() -> Vec<f32>;

/// Retrieve the index buffer for the last parsed layer.
///
/// Returns a **copy** of the indices as a new Uint32Array.
/// Safe to transfer to main thread.
#[wasm_bindgen]
pub fn get_indices() -> Vec<u32>;
```

### 8.2 Why `Vec<f32>` Instead of `Float32Array` View

The feasibility doc proposed zero-copy `Float32Array` views into WASM memory. After accounting for the Web Worker architecture, this is revised:

| Approach | Problem with Workers |
|----------|---------------------|
| `Float32Array` view into WASM memory | Cannot be `Transferred` — it's a view, not an owned buffer. Would require a copy anyway before `postMessage`. |
| `Vec<f32>` returned via wasm-bindgen | wasm-bindgen automatically creates a new JS-owned `ArrayBuffer` by copying. This buffer can then be `Transferred` to main thread zero-copy. |

**Net result:** One copy (WASM → Worker JS), then zero-copy transfer (Worker → Main). Total: one copy. The alternative (view + manual copy + transfer) is also one copy but with more fragile code.

### 8.3 Internal State

```rust
// Module-level state (single-threaded WASM, so this is safe).
thread_local! {
    static LAST_GEOMETRY: RefCell<Option<LayerGeometry>> = RefCell::new(None);
}

fn store_geometry(geom: LayerGeometry) {
    LAST_GEOMETRY.with(|g| {
        *g.borrow_mut() = Some(geom);
    });
}
```

This pattern exists so that `parse_gerber()` can store the result and `get_positions()` / `get_indices()` can retrieve it without re-parsing. The caller sequence is always: `parse → get_positions → get_indices → parse next file`.

### 8.4 Rust Internal Architecture

```
lib.rs
  │
  ├── parse_gerber(bytes)
  │     │
  │     ▼
  │   gerber_parser::parse(BufReader::new(Cursor::new(bytes)))
  │     │
  │     ▼
  │   GerberDoc { commands, apertures, units, format }
  │     │
  │     ▼
  │   geometry::convert(doc) → LayerGeometry
  │     │
  │     ├── Walk commands sequentially, maintain GerberState
  │     │
  │     ├── D03 Flash ──► aperture::expand(aperture, position)
  │     ├── D01 Linear ─► stroke::widen(from, to, aperture)
  │     ├── D01 Arc ────► arc::tessellate(from, to, center, dir)
  │     │                  └► stroke::widen per segment
  │     ├── G36 Region ─► collect points until G37
  │     │                  └► region::triangulate(points)
  │     ├── LPC Clear ──► polarity::mark_clear(geometry)
  │     ├── SR Block ───► step_repeat::duplicate(geometry, grid)
  │     └── AM Macro ───► macro_eval::evaluate(macro_def, params)
  │     │
  │     ▼
  │   LayerGeometry { positions, indices, bounds, ... }
  │     │
  │     ▼
  │   store_geometry(result)
  │   return LayerMeta via serde-wasm-bindgen
  │
  ├── get_positions() → Vec<f32> from stored geometry
  └── get_indices() → Vec<u32> from stored geometry
```

---

## 9. Rendering Pipeline

The renderer uses an **on-demand** strategy: it only draws a frame when something has changed.

### 9.1 Dirty Flag System

```typescript
export class Renderer {
  private dirty = true;
  private rafId: number | null = null;

  /** Mark the scene as needing a redraw. */
  markDirty(): void {
    if (!this.dirty) {
      this.dirty = true;
      this.scheduleFrame();
    }
  }

  private scheduleFrame(): void {
    if (this.rafId === null) {
      this.rafId = requestAnimationFrame(() => this.renderFrame());
    }
  }

  private renderFrame(): void {
    this.rafId = null;
    if (!this.dirty) return;
    this.dirty = false;
    this.draw();
  }
}
```

**Key properties:**
- At most one `requestAnimationFrame` is pending at any time.
- Multiple `markDirty()` calls between frames are coalesced.
- Zero CPU/GPU usage when the view is static.
- During continuous interaction (drag), `markDirty()` is called per mouse event, resulting in 60fps drawing.

### 9.2 What Triggers Dirty

| Event | Triggers dirty? |
|-------|----------------|
| `viewState` signal changes (zoom, pan) | Yes |
| `layers` signal changes (new layer added) | Yes |
| Layer visibility toggled | Yes |
| `globalOpacity` changes | Yes |
| Canvas resizes | Yes (also updates viewport) |
| Mouse move (cursor coord display) | No (UI-only update, no re-render) |
| WebGL context restored | Yes (after VBO re-upload) |

### 9.3 Draw Sequence

```
draw()
│
├── gl.viewport(0, 0, canvas.width, canvas.height)
├── gl.clear(COLOR_BUFFER_BIT)
│
├── Compute view matrix from ViewState + canvas aspect ratio
│     viewMatrix = computeViewMatrix(viewState, canvasWidth, canvasHeight)
│
├── gl.useProgram(shaderProgram)
├── gl.uniformMatrix3fv(u_viewMatrix, false, viewMatrix)
│
├── for each LayerNode in scene.getVisibleLayers():  // z-order, back to front
│     │
│     ├── if node.renderState is null: skip
│     ├── gl.bindBuffer(GL.ARRAY_BUFFER, node.renderState.positionVBO)
│     ├── gl.vertexAttribPointer(a_position, 2, FLOAT, false, 0, 0)
│     ├── gl.enableVertexAttribArray(a_position)
│     ├── gl.bindBuffer(GL.ELEMENT_ARRAY_BUFFER, node.renderState.indexVBO)
│     │
│     ├── Compute final color: [r, g, b, a * node.opacity * globalOpacity]
│     ├── gl.uniform4fv(u_color, finalColor)
│     │
│     └── gl.drawElements(GL.TRIANGLES, node.renderState.indexCount,
│                          GL.UNSIGNED_INT, 0)
│
└── Overlays:
      for each OverlayNode in scene.overlays.children:
        if node.visible: node.renderFn(gl, viewMatrix)
```

### 9.4 View Matrix Computation

The view matrix is a 3x3 affine matrix (column-major for WebGL) that transforms board coordinates to clip space.

```
Given:
  center = (cx, cy)     // board-space center of view
  zoom   = z            // scale factor (1.0 = fit-to-view)
  canvasW, canvasH      // canvas pixel dimensions
  boardBounds           // bounding box of all layers

Step 1: Compute scale to fit board in canvas (base scale)
  scaleX = canvasW / (boardBounds.width * (1 + padding))
  scaleY = canvasH / (boardBounds.height * (1 + padding))
  baseScale = min(scaleX, scaleY)

Step 2: Apply user zoom
  finalScale = baseScale * zoom

Step 3: Convert center to NDC offset
  offsetX = -cx * finalScale * 2 / canvasW
  offsetY = -cy * finalScale * 2 / canvasH

Step 4: Build matrix (column-major)
  ┌                                    ┐
  │ finalScale*2/W    0         0      │
  │ 0         finalScale*2/H    0      │
  │ offsetX          offsetY    1      │
  └                                    ┘
```

### 9.5 Shader Programs

Only one shader program for MVP:

**Vertex Shader:**
```glsl
attribute vec2 a_position;
uniform mat3 u_viewMatrix;

void main() {
    vec3 pos = u_viewMatrix * vec3(a_position, 1.0);
    gl_Position = vec4(pos.xy, 0.0, 1.0);
}
```

**Fragment Shader:**
```glsl
precision mediump float;
uniform vec4 u_color;

void main() {
    gl_FragColor = u_color;
}
```

Future shaders (overlays) can be added without modifying the layer shader.

### 9.6 WebGL Context Loss/Restore

```typescript
canvas.addEventListener("webglcontextlost", (e) => {
  e.preventDefault();
  store.appState.value = AppState.Error;
  store.error.value = { code: "WEBGL_CONTEXT_LOST", message: "..." };
});

canvas.addEventListener("webglcontextrestored", () => {
  renderer.recompileShaders();
  sceneManager.restoreGPUResources();  // re-upload all VBOs
  store.appState.value = AppState.Rendered;
  store.error.value = null;
  renderer.markDirty();
});
```

---

## 10. Geometry Pipeline (Rust)

This section details the internal architecture of `crates/gerberview-wasm/src/geometry/`.

### 10.1 Pipeline Overview

```
GerberDoc
  │
  ▼
┌──────────────────────────────────────────────────────────┐
│  Geometry Converter (geometry/mod.rs)                     │
│                                                          │
│  State Machine (GerberState):                            │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ current_point: Point                                │ │
│  │ current_aperture: Option<i32>                       │ │
│  │ interpolation_mode: InterpolationMode               │ │
│  │ polarity: Polarity                                  │ │
│  │ region_mode: bool                                   │ │
│  │ region_points: Vec<Point>                           │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                          │
│  Command dispatch:                                       │
│  ┌──────────────┬───────────────────────────────────┐    │
│  │ Command      │ Handler                           │    │
│  ├──────────────┼───────────────────────────────────┤    │
│  │ SelectAp(n)  │ state.current_aperture = n        │    │
│  │ D01          │ → stroke.rs / arc.rs              │    │
│  │ D02          │ state.current_point = target      │    │
│  │ D03          │ → aperture.rs                     │    │
│  │ G01          │ state.interp = Linear             │    │
│  │ G02          │ state.interp = CW                 │    │
│  │ G03          │ state.interp = CCW                │    │
│  │ G36          │ state.region_mode = true           │    │
│  │ G37          │ → region.rs, state.region = false  │    │
│  │ LP(Dark)     │ state.polarity = Dark             │    │
│  │ LP(Clear)    │ → polarity.rs                     │    │
│  │ SR(...)      │ → step_repeat.rs                  │    │
│  │ AM(...)      │ → macro_eval.rs                   │    │
│  └──────────────┴───────────────────────────────────┘    │
│                                                          │
│  Output accumulator (GeometryBuilder):                   │
│  ┌─────────────────────────────────────────────────────┐ │
│  │ positions: Vec<f32>                                 │ │
│  │ indices: Vec<u32>                                   │ │
│  │ bounds: BoundingBox (updated per vertex)            │ │
│  │ warnings: Vec<String>                               │ │
│  │                                                     │ │
│  │ push_vertex(x, y) → u32 (index)                    │ │
│  │ push_triangle(a, b, c)                              │ │
│  │ push_quad(a, b, c, d)                               │ │
│  │ current_vertex_count() → u32                        │ │
│  └─────────────────────────────────────────────────────┘ │
│                                                          │
└──────────────────────────────────────────────────────────┘
  │
  ▼
LayerGeometry { positions, indices, bounds, ... }
```

### 10.2 GeometryBuilder

The `GeometryBuilder` is passed by mutable reference to all geometry functions. It accumulates vertices and indices in a single pair of `Vec`s, avoiding intermediate allocations.

```rust
pub struct GeometryBuilder {
    positions: Vec<f32>,
    indices: Vec<u32>,
    bounds: BoundingBox,
    warnings: Vec<String>,
}

impl GeometryBuilder {
    /// Add a vertex, return its index.
    pub fn push_vertex(&mut self, x: f64, y: f64) -> u32;

    /// Add a triangle from three vertex indices.
    pub fn push_triangle(&mut self, a: u32, b: u32, c: u32);

    /// Add a quad as two triangles (a-b-c, a-c-d).
    pub fn push_quad(&mut self, a: u32, b: u32, c: u32, d: u32);

    /// Add N-gon centered at (cx, cy) with given radius and segment count.
    pub fn push_ngon(&mut self, cx: f64, cy: f64, radius: f64, segments: u32) -> u32;

    /// Record a warning message.
    pub fn warn(&mut self, msg: String);

    /// Finalize into LayerGeometry.
    pub fn build(self) -> LayerGeometry;
}
```

### 10.3 Module Interfaces (Rust)

Each geometry sub-module has a pure function signature:

```rust
// aperture.rs
pub fn flash_aperture(
    builder: &mut GeometryBuilder,
    aperture: &Aperture,
    position: Point,
) -> Result<(), GeometryError>;

// stroke.rs
pub fn draw_linear(
    builder: &mut GeometryBuilder,
    from: Point,
    to: Point,
    aperture: &Aperture,
) -> Result<(), GeometryError>;

// arc.rs
pub fn draw_arc(
    builder: &mut GeometryBuilder,
    from: Point,
    to: Point,
    center_offset: Point,
    direction: ArcDirection,
    aperture: &Aperture,
) -> Result<(), GeometryError>;

// region.rs
pub fn fill_region(
    builder: &mut GeometryBuilder,
    boundary: &[Point],
) -> Result<(), GeometryError>;

// step_repeat.rs
pub fn apply_step_repeat(
    builder: &mut GeometryBuilder,
    block_geometry: &LayerGeometry,
    repeat_x: u32,
    repeat_y: u32,
    step_x: f64,
    step_y: f64,
) -> Result<(), GeometryError>;

// macro_eval.rs
pub fn evaluate_macro(
    builder: &mut GeometryBuilder,
    macro_def: &ApertureMacro,
    params: &[f64],
    position: Point,
) -> Result<(), GeometryError>;

// polarity.rs
pub fn set_clear_color(builder: &mut GeometryBuilder, background: [f32; 4]);
```

Every function takes `&mut GeometryBuilder` as first argument. No function allocates its own buffers. This makes memory usage predictable and avoids fragmentation.

---

## 11. Data Flow — End to End

### 11.1 Complete Sequence: File Drop to Rendered Board

```
User drops board.zip
       │
       ▼
[Main: zip-handler.ts]
  1. Validate: is it a .zip? Is it < 100MB?
  2. JSZip.loadAsync(file) → entries
  3. For each entry:
       layer-identify.ts → { fileName, layerType, fileType }
  4. Filter: keep gerber + excellon only
  5. Read each file as Uint8Array
       │
       ▼
[Main: store]
  6. appState.value = "loading"
  7. loadingProgress.value = { current: 0, total: N }
       │
       ▼
[Main → Worker: postMessage]
  8. Send ParseRequestMessage
     { type: "parse-request", requestId, files: [ { fileName, layerType,
       fileType, content: ArrayBuffer } ] }
     Transfer: all ArrayBuffers
       │
       ▼
[Worker: parse-worker.ts]
  9.  For each file in message.files:
        a. If fileType === "gerber":
             meta = wasm.parse_gerber(new Uint8Array(content))
             positions = wasm.get_positions()  → Float32Array
             indices = wasm.get_indices()      → Uint32Array
        b. If fileType === "excellon":
             meta = wasm.parse_excellon(new Uint8Array(content))
             positions = wasm.get_positions()
             indices = wasm.get_indices()
        c. Post result: { type: "layer-result", positions, indices, meta, ... }
           Transfer: positions.buffer, indices.buffer
 10.  Post: { type: "parse-complete", ... }
       │
       ▼
[Main: onmessage handler]
 11.  For each "layer-result":
        a. Create ParsedLayer { ..., positionBuffer, indexBuffer }
        b. store.layers.update(layers => [...layers, newLayer])
        c. store.loadingProgress.update(p => { current: p.current + 1, ... })
       │
       ▼
[Main: store subscription → SceneManager]
 12.  SceneManager.addLayer(parsedLayer):
        a. gl.createBuffer() → positionVBO
        b. gl.bindBuffer(ARRAY_BUFFER, positionVBO)
        c. gl.bufferData(ARRAY_BUFFER, positionBuffer, STATIC_DRAW)
        d. Repeat for indexVBO
        e. Create LayerNode, insert in scene at correct z-order
       │
       ▼
[Main: store subscription → Renderer]
 13.  renderer.markDirty()
       │
       ▼
[Main: requestAnimationFrame]
 14.  Renderer.draw():
        a. Clear canvas
        b. Compute view matrix
        c. For each visible LayerNode (back-to-front):
             Bind VBO, set color uniform, drawElements
       │
       ▼
 15.  Board is visible on screen.

[Main: store subscription → UI]
 16.  After "parse-complete":
        a. appState.value = "rendered"
        b. Layer panel populated from store.layers
        c. Status bar shows stats
        d. Fit-to-view computed and applied
```

### 11.2 Interaction Sequence: Zoom

```
User scrolls mouse wheel
       │
       ▼
[Main: interaction.ts]
  1. wheelHandler(event):
       a. Compute cursor position in board coords
       b. Compute new zoom = old zoom * (1 ± zoomFactor)
       c. Clamp to [minZoom, maxZoom]
       d. Adjust center so cursor point stays fixed:
            newCenter = cursorPos + (oldCenter - cursorPos) * (oldZoom / newZoom)
       e. store.viewState.value = { center: newCenter, zoom: newZoom }
       │
       ▼
[Main: store subscription]
  2. viewState changed → renderer.markDirty()
       │
       ▼
[Main: requestAnimationFrame]
  3. Renderer.draw() with new view matrix
```

---

## 12. Memory Management

### 12.1 Memory Lifecycle by Phase

```
Phase 1: File Upload
  ┌────────────────────────────────────────────┐
  │ file: ArrayBuffer (owned by main thread)   │ ← User's file
  │ zipEntries: Uint8Array[] (JSZip output)    │ ← Extracted files
  └─────────────────────┬──────────────────────┘
                        │ Transferred to Worker (main loses ownership)
                        ▼
Phase 2: Parsing (Worker)
  ┌────────────────────────────────────────────┐
  │ content: ArrayBuffer (transferred in)      │ ← Owned by Worker
  │ WASM heap: gerber_parser internals         │ ← Transient during parse
  │ positions: Vec<f32> → Float32Array (copy)  │ ← New JS buffer
  │ indices: Vec<u32> → Uint32Array (copy)     │ ← New JS buffer
  └─────────────────────┬──────────────────────┘
                        │ positions/indices Transferred to Main
                        │ content freed (no longer needed)
                        ▼
Phase 3: GPU Upload (Main)
  ┌────────────────────────────────────────────┐
  │ positionBuffer: Float32Array (owned)       │ ← Transferred from Worker
  │ indexBuffer: Uint32Array (owned)           │ ← Transferred from Worker
  │   → gl.bufferData() copies to GPU VRAM    │
  │   → positionBuffer can be released         │
  │   → indexBuffer can be released            │
  │                                            │
  │ GPU: positionVBO, indexVBO                 │ ← Owned by WebGL context
  └────────────────────────────────────────────┘

Phase 4: Rendering (Main, ongoing)
  ┌────────────────────────────────────────────┐
  │ SceneGraph holds VBO handles               │
  │ No JS-side copies of vertex data needed    │
  │ Only metadata (LayerMeta) retained in RAM  │
  └────────────────────────────────────────────┘
```

### 12.2 Memory Release Strategy

| Event | What to release | How |
|-------|----------------|-----|
| Layer parsed and uploaded to GPU | `positionBuffer`, `indexBuffer` (JS arrays) | Set to `null`, let GC collect |
| New file loaded (replace previous) | All VBOs, scene nodes, layer data | `SceneManager.clear()` → `gl.deleteBuffer()` for each VBO |
| WebGL context lost | VBO handles are invalid | `SceneManager.releaseGPUResources()` — null out handles |
| WebGL context restored | Re-upload from... | Problem: JS arrays were released. Solution: keep `positionBuffer`/`indexBuffer` in `ParsedLayer` until rendering is stable. Release only after confirming context is healthy for 5s. |

### 12.3 Memory Budget

| Component | Typical 6-layer board | Worst case (complex 10-layer) |
|-----------|----------------------|------------------------------|
| WASM heap (transient during parse) | ~2-8 MB | ~20 MB |
| Position buffers (all layers, JS) | ~2-4 MB | ~10 MB |
| Index buffers (all layers, JS) | ~1-2 MB | ~5 MB |
| GPU VRAM (VBOs) | ~3-6 MB | ~15 MB |
| Scene graph metadata | ~1 KB | ~2 KB |
| Store state | ~1 KB | ~2 KB |
| **Total RAM (peak, during upload)** | **~10-20 MB** | **~50 MB** |
| **Total RAM (steady state)** | **~5-10 MB** | **~20 MB** |

Well within the NFR-105 budget of 128 MB.

---

## 13. Error Propagation Architecture

```
RUST (WASM)                    WORKER (JS)                     MAIN (JS)
───────────                    ───────────                     ─────────

gerber_parser                  parse-worker.ts                 onmessage
returns Err(ParseError)        │                               handler
         │                     │                               │
         ▼                     │                               │
thiserror enum                 │                               │
  ParseError::                 │                               │
  InvalidCommand(..)           │                               │
         │                     │                               │
         ▼                     │                               │
map_err(|e| {                  │                               │
  JsValue::from_str(           │                               │
    &format!("{e}")            │                               │
  )                            │                               │
})                             │                               │
         │                     │                               │
         ▼                     │                               │
Err(JsValue) ────────────────► try { parse_gerber(bytes) }     │
                               catch (e) {                     │
                                 postMessage({                 │
                                   type: "layer-error",        │
                                   error: String(e) ──────────► AppError {
                                 })                            │   code: PARSE_FAILED,
                               }                               │   message: user-friendly,
                                                               │   details: raw error
                                                               │ }
                                                               │       │
                                                               │       ▼
                                                               │ store.error.value = appError
                                                               │       │
                                                               │       ▼
                                                               │ UI.errorBanner renders
                                                               │ console.error(details)
```

### 13.1 Error Domains

```typescript
/** Maps raw error strings to user-friendly messages. */
function toAppError(rawError: string, fileName: string): AppError {
  if (rawError.includes("IO error")) {
    return { code: ErrorCode.ParseFailed,
             message: `Failed to read "${fileName}".`,
             details: rawError };
  }
  if (rawError.includes("invalid aperture")) {
    return { code: ErrorCode.ParseFailed,
             message: `"${fileName}" contains an invalid aperture definition.`,
             details: rawError };
  }
  // ... more patterns ...
  return { code: ErrorCode.ParseFailed,
           message: `Failed to parse "${fileName}".`,
           details: rawError };
}
```

### 13.2 Partial Failure Strategy

If 5 of 7 layers parse successfully and 2 fail:
1. The 5 successful layers are rendered normally.
2. The 2 failed layers are listed in the layer panel with a warning icon.
3. A non-blocking error banner shows: "2 of 7 layers failed to parse."
4. The user can click each failed layer to see the specific error.
5. `appState` = `Rendered` (not `Error`), because the board is partially viewable.

`appState` = `Error` is reserved for total failures (e.g., no layers parsed, WebGL unavailable, WASM load failed).

---

## 14. Module Dependency Rules

Strict dependency rules prevent spaghetti coupling. Enforced by import discipline (linting or manual review).

```
                    ┌──────────┐
                    │  types   │  ← Depended on by ALL modules
                    │constants │     (never depends on anything)
                    └────┬─────┘
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
     ┌─────────┐  ┌───────────┐  ┌──────────────┐
     │ signal  │  │  layer-   │  │  parse-worker │
     │         │  │ identify  │  │ (wasm-bridge) │
     └────┬────┘  └───────────┘  └──────┬───────┘
          │                             │
          ▼                             │
     ┌─────────┐                        │
     │  store  │                        │
     └────┬────┘                        │
          │                             │
     ┌────┴────────────────────┐        │
     ▼                         ▼        ▼
┌─────────┐            ┌──────────┐
│  scene  │            │ zip-     │
│         │            │ handler  │
└────┬────┘            └──────────┘
     │
     ▼
┌──────────┐
│ renderer │
└────┬─────┘
     │
     ▼
┌──────────────┐
│ interaction  │
└──────────────┘
     │
     ▼
┌─────┐
│ ui  │  ← depends on store, scene (reads), interaction (coordinates)
└─────┘
     │
     ▼
┌──────┐
│ main │  ← wires everything together (composition root)
└──────┘
```

### 14.1 Import Rules

| Module | MAY import | MUST NOT import |
|--------|-----------|----------------|
| `types.ts`, `constants.ts` | Nothing (leaf modules) | Everything |
| `signal.ts` | `types` | Any other module |
| `store.ts` | `signal`, `types` | Any UI, renderer, or worker module |
| `layer-identify.ts` | `types`, `constants` | Store, scene, renderer |
| `parse-worker.ts` | WASM module, `types` | Store, scene, renderer, DOM |
| `zip-handler.ts` | JSZip, `layer-identify`, `types` | Store, renderer |
| `scene.ts` | `types`, `constants` | Store, renderer |
| `renderer.ts` | `scene`, `types`, `constants` | Store, worker, zip-handler |
| `interaction.ts` | `store`, `renderer`, `types` | Worker, zip-handler |
| `ui.ts` | `store`, `scene`, `types`, `constants` | Renderer internals, worker |
| `main.ts` | Everything (composition root) | N/A |

### 14.2 Rust Module Dependency Rules

```
lib.rs
  ├── geometry/
  │     ├── types.rs      ← Depended on by all geometry modules
  │     ├── aperture.rs   ← depends on types
  │     ├── stroke.rs     ← depends on types
  │     ├── arc.rs        ← depends on types, stroke (for segment widening)
  │     ├── region.rs     ← depends on types, earclip
  │     ├── polarity.rs   ← depends on types
  │     ├── step_repeat.rs ← depends on types
  │     ├── macro_eval.rs  ← depends on types, aperture (for primitives)
  │     └── mod.rs         ← orchestrates all sub-modules
  └── excellon/
        ├── types.rs      ← DrillHole, ToolDefinition
        ├── parser.rs     ← depends on types
        └── mod.rs
```

No geometry sub-module may import from `excellon/`. No `excellon/` module may import from `geometry/`. Both expose types through their `mod.rs` and are consumed only by `lib.rs`.

---

## 15. Updated Project Structure

This supersedes Section 4 of the spec to reflect the Web Worker, reactive store, and scene graph additions.

```
gerberview/
├── turbo.json
├── package.json
├── pnpm-workspace.yaml
├── .commitlintrc.json
├── .husky/
│   ├── pre-commit
│   └── commit-msg
├── .github/workflows/
│   ├── ci.yml
│   └── deploy.yml
├── rustfmt.toml
├── clippy.toml
├── deny.toml
├── Cargo.toml                              # Virtual manifest
├── README.md
├── LICENSE
├── .gitignore
│
├── crates/
│   └── gerberview-wasm/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs                      # wasm_bindgen exports + thread_local state
│       │   ├── error.rs                    # GeometryError, WasmError enums
│       │   ├── geometry/
│       │   │   ├── mod.rs                  # convert(GerberDoc) → LayerGeometry
│       │   │   ├── types.rs                # Point, BoundingBox, LayerGeometry, GeometryBuilder
│       │   │   ├── aperture.rs
│       │   │   ├── stroke.rs
│       │   │   ├── arc.rs
│       │   │   ├── region.rs
│       │   │   ├── polarity.rs
│       │   │   ├── macro_eval.rs
│       │   │   └── step_repeat.rs
│       │   └── excellon/
│       │       ├── mod.rs
│       │       ├── parser.rs
│       │       └── types.rs
│       ├── tests/
│       │   ├── parse_test.rs
│       │   ├── geometry_test.rs
│       │   ├── excellon_test.rs
│       │   └── fixtures/
│       │       ├── kicad-sample/
│       │       ├── arduino-uno/
│       │       └── eagle-sample/
│       └── benches/
│           └── parse_bench.rs
│
├── apps/
│   └── web/
│       ├── package.json
│       ├── tsconfig.json
│       ├── vite.config.ts
│       ├── tailwind.config.ts
│       ├── postcss.config.js
│       ├── index.html
│       ├── public/
│       │   ├── sw.js
│       │   └── favicon.svg
│       ├── src/
│       │   ├── main.ts                     # Composition root: wires all modules
│       │   ├── types.ts                    # All TS types (spec Section 7.3 + worker messages)
│       │   ├── constants.ts                # Colors, limits, z-order, config
│       │   │
│       │   ├── core/                       # State & reactivity (no DOM dependency)
│       │   │   ├── signal.ts               # Signal<T>, Computed<T> primitives
│       │   │   └── store.ts                # AppStore: all application signals
│       │   │
│       │   ├── engine/                     # Parsing & file handling (no DOM dependency)
│       │   │   ├── parse-worker.ts         # Web Worker entry point (runs in Worker)
│       │   │   ├── worker-client.ts        # Main-thread wrapper: postMessage + onmessage
│       │   │   ├── zip-handler.ts          # ZIP extraction + validation
│       │   │   └── layer-identify.ts       # Filename → LayerType
│       │   │
│       │   ├── scene/                      # Scene graph (depends on WebGL, not DOM)
│       │   │   ├── scene.ts                # SceneRoot, SceneManager
│       │   │   └── nodes.ts                # LayerNode, BoardNode, OverlayNode types
│       │   │
│       │   ├── render/                     # WebGL rendering
│       │   │   ├── renderer.ts             # Dirty-flag renderer, draw loop
│       │   │   ├── shader.ts               # Shader compilation utilities
│       │   │   └── shaders/
│       │   │       ├── vertex.glsl
│       │   │       └── fragment.glsl
│       │   │
│       │   ├── interaction/                # User input handling
│       │   │   ├── interaction.ts          # Mouse/keyboard/touch dispatcher
│       │   │   ├── zoom.ts                 # Zoom logic (cursor-centered)
│       │   │   ├── pan.ts                  # Pan logic (click-drag)
│       │   │   └── touch.ts                # Touch gesture handling
│       │   │
│       │   └── ui/                         # DOM-dependent UI modules
│       │       ├── ui.ts                   # Orchestrator: layer panel, upload zone, status bar
│       │       ├── upload-zone.ts          # Drag-drop + file picker component
│       │       ├── layer-panel.ts          # Layer checkboxes, color swatches
│       │       ├── status-bar.ts           # Stats, coordinates, dimensions
│       │       └── error-banner.ts         # Error display component
│       │
│       ├── __tests__/
│       │   ├── signal.test.ts
│       │   ├── store.test.ts
│       │   ├── layer-identify.test.ts
│       │   ├── zip-handler.test.ts
│       │   ├── zoom.test.ts
│       │   ├── pan.test.ts
│       │   ├── scene.test.ts
│       │   └── renderer.test.ts
│       │
│       └── e2e/
│           ├── playwright.config.ts
│           ├── fixtures/
│           └── tests/
│               ├── upload.spec.ts
│               ├── rendering.spec.ts
│               ├── interaction.spec.ts
│               ├── layers.spec.ts
│               ├── error-states.spec.ts
│               └── accessibility.spec.ts
│
└── packages/
    └── eslint-config/
        ├── package.json
        └── index.js
```

### 15.1 Key Structural Changes from Spec

| Change | Rationale |
|--------|-----------|
| `src/core/` directory for `signal.ts`, `store.ts` | Separates reactive primitives from everything else |
| `src/engine/` directory for worker, zip, layer-identify | Groups all non-rendering computation |
| `src/scene/` directory for scene graph | Clean boundary between data model and renderer |
| `src/render/` directory for WebGL | Renderer is a pure consumer of scene graph |
| `src/interaction/` split into zoom, pan, touch | Single-responsibility per gesture type |
| `src/ui/` split into upload-zone, layer-panel, status-bar, error-banner | One file per DOM component |
| `worker-client.ts` added | Encapsulates Worker postMessage/onmessage behind a typed async API |
| `error.rs` added to Rust crate | Centralizes error types |

---

## 16. Component Catalog

### 16.1 TypeScript Components

| Component | File | Responsibility | Key Dependencies |
|-----------|------|---------------|-----------------|
| **Signal** | `core/signal.ts` | Reactive primitive: hold value, notify subscribers | None |
| **AppStore** | `core/store.ts` | All application state as signals | `signal.ts`, `types.ts` |
| **WorkerClient** | `engine/worker-client.ts` | Typed async API over Worker postMessage | `types.ts` |
| **ParseWorker** | `engine/parse-worker.ts` | Worker entry: loads WASM, dispatches parse calls | WASM module |
| **ZipHandler** | `engine/zip-handler.ts` | Extract ZIP, validate, identify files | JSZip, `layer-identify.ts` |
| **LayerIdentify** | `engine/layer-identify.ts` | Filename pattern matching → LayerType | `constants.ts` |
| **SceneManager** | `scene/scene.ts` | Manage scene nodes, VBO lifecycle, z-order | WebGL context |
| **Renderer** | `render/renderer.ts` | Dirty-flag render loop, shader management | `scene.ts`, WebGL context |
| **Interaction** | `interaction/interaction.ts` | Dispatch mouse/keyboard/touch to handlers | `store.ts`, `renderer.ts` |
| **UploadZone** | `ui/upload-zone.ts` | Drag-drop + click file picker | DOM |
| **LayerPanel** | `ui/layer-panel.ts` | Layer toggles, color swatches | `store.ts` |
| **StatusBar** | `ui/status-bar.ts` | Stats, coordinates, dimensions | `store.ts` |
| **ErrorBanner** | `ui/error-banner.ts` | Dismissable error display | `store.ts` |
| **Main** | `main.ts` | Composition root: instantiate and wire | All |

### 16.2 Rust Components

| Component | File | Responsibility |
|-----------|------|---------------|
| **WASM Bridge** | `lib.rs` | `#[wasm_bindgen]` exports, thread_local state |
| **Geometry Converter** | `geometry/mod.rs` | Walk GerberDoc commands, dispatch to sub-modules |
| **GeometryBuilder** | `geometry/types.rs` | Accumulate vertices + indices |
| **Aperture Expander** | `geometry/aperture.rs` | Flash shapes: circle, rect, obround, polygon |
| **Stroke Widener** | `geometry/stroke.rs` | D01 linear draws → quads + endcaps |
| **Arc Tessellator** | `geometry/arc.rs` | G02/G03 arcs → line segments → widened strokes |
| **Region Filler** | `geometry/region.rs` | G36/G37 polygons → earclip triangulation |
| **Polarity Handler** | `geometry/polarity.rs` | LPD/LPC state + clear color marking |
| **Step Repeat** | `geometry/step_repeat.rs` | SR block vertex duplication |
| **Macro Evaluator** | `geometry/macro_eval.rs` | AM primitive evaluation → vertices |
| **Excellon Parser** | `excellon/parser.rs` | Drill file parsing → DrillHole list |
| **Error Types** | `error.rs` | GeometryError, WasmError enums |

---

## 17. Design Patterns

| Pattern | Where Used | Purpose |
|---------|-----------|---------|
| **Observer (Signals)** | `core/signal.ts` | Reactive state propagation without framework |
| **Builder** | `geometry/types.rs` (`GeometryBuilder`) | Accumulate geometry incrementally |
| **State Machine** | `geometry/mod.rs` (`GerberState`) | Track Gerber interpreter state across commands |
| **Mediator** | `main.ts` | Composition root wires all modules, no module talks to others directly |
| **Strategy** | `geometry/aperture.rs` | Different vertex generation per aperture type |
| **Facade** | `engine/worker-client.ts` | Hides Worker postMessage/onmessage behind async API |
| **Dirty Flag** | `render/renderer.ts` | Coalesce multiple state changes into single frame |
| **Transfer Object** | Worker messages | Typed DTOs cross the Worker boundary |
| **Scene Graph** | `scene/scene.ts` | Decouple data model from renderer |
| **Null Object** | `SceneRoot.board = null` | No-board state handled uniformly (render empty scene) |

### 17.1 WorkerClient Facade

```typescript
export class WorkerClient {
  private worker: Worker;
  private pendingRequestId: string | null = null;

  constructor() {
    this.worker = new Worker(
      new URL("./parse-worker.ts", import.meta.url),
      { type: "module" },
    );
  }

  /** Wait for worker to initialize WASM. */
  async waitForReady(): Promise<void>;

  /**
   * Parse files and stream results.
   * Cancels any in-flight request.
   * Returns an async iterable of layer results.
   */
  async *parseFiles(
    files: readonly IdentifiedFile[],
  ): AsyncGenerator<LayerResultMessage | LayerErrorMessage>;

  /** Cancel any in-flight parse. */
  cancel(): void;

  /** Terminate the worker. */
  dispose(): void;
}
```

The `AsyncGenerator` pattern lets the caller process layers as they arrive, enabling progressive rendering (each layer appears as it's parsed, rather than waiting for all layers).

### 17.2 Composition Root (`main.ts`)

```typescript
async function main(): Promise<void> {
  // 1. Create store
  const store = createAppStore();

  // 2. Get canvas, init WebGL
  const canvas = document.getElementById("canvas") as HTMLCanvasElement;
  const gl = canvas.getContext("webgl", { alpha: false, antialias: false });
  if (!gl) { /* error path */ }
  gl.getExtension("OES_element_index_uint");

  // 3. Create scene, renderer
  const sceneManager = new SceneManager(gl);
  const renderer = new Renderer(gl, sceneManager, store);

  // 4. Create worker client
  const workerClient = new WorkerClient();
  await workerClient.waitForReady();

  // 5. Wire interaction
  setupInteraction(canvas, store, renderer);

  // 6. Wire UI
  setupUI(store, sceneManager);

  // 7. Wire upload handler
  setupUpload(store, workerClient, sceneManager, renderer);

  // 8. Subscribe: viewState/opacity/layers changes → renderer.markDirty()
  store.viewState.subscribe(() => renderer.markDirty());
  store.globalOpacity.subscribe(() => renderer.markDirty());
}
```

No module except `main.ts` knows about all other modules. Every other module receives its dependencies via constructor arguments or function parameters.

---

## 18. Key Algorithms

### 18.1 Cursor-Centered Zoom

```
Input:
  cursorScreen = (mouseX, mouseY) in pixels
  cursorBoard  = screenToBoard(cursorScreen, currentViewState, canvas)
  zoomDelta    = +1 (zoom in) or -1 (zoom out)
  zoomFactor   = 1.15 (15% per step)

Algorithm:
  newZoom = currentZoom * pow(zoomFactor, zoomDelta)
  newZoom = clamp(newZoom, minZoom, maxZoom)

  // Keep the board point under the cursor fixed:
  // Before zoom: cursorBoard maps to cursorScreen
  // After zoom:  cursorBoard must still map to cursorScreen
  // Solve for newCenter:
  newCenter.x = cursorBoard.x - (cursorBoard.x - currentCenter.x) * (currentZoom / newZoom)
  newCenter.y = cursorBoard.y - (cursorBoard.y - currentCenter.y) * (currentZoom / newZoom)
```

### 18.2 Screen ↔ Board Coordinate Conversion

```
boardToScreen(boardPoint, viewState, canvas):
  sx = (boardPoint.x - viewState.centerX) * scale + canvas.width / 2
  sy = canvas.height / 2 - (boardPoint.y - viewState.centerY) * scale
  where scale = baseScale * viewState.zoom

screenToBoard(screenPoint, viewState, canvas):
  bx = (screenPoint.x - canvas.width / 2) / scale + viewState.centerX
  by = (canvas.height / 2 - screenPoint.y) / scale + viewState.centerY
```

### 18.3 Fit-to-View

```
Input: boardBounds (union of all layer bounds), canvas dimensions

Algorithm:
  boardWidth  = bounds.maxX - bounds.minX
  boardHeight = bounds.maxY - bounds.minY
  padding     = 0.05  (5% margin on each side)

  scaleX = canvas.width  / (boardWidth  * (1 + 2 * padding))
  scaleY = canvas.height / (boardHeight * (1 + 2 * padding))
  fitScale = min(scaleX, scaleY)

  center.x = (bounds.minX + bounds.maxX) / 2
  center.y = (bounds.minY + bounds.maxY) / 2

  viewState = { centerX: center.x, centerY: center.y, zoom: 1.0 }
  baseScale = fitScale   (stored separately, zoom=1.0 means fit-to-view)
```

### 18.4 Arc Center Computation (Rust)

```
Given:
  start  = current_point
  end    = target point
  I, J   = center offset from start (Gerber I/J parameters)

Center:
  center = (start.x + I, start.y + J)

Radius:
  r = sqrt((start.x - center.x)² + (start.y - center.y)²)

  // Validate: end point should also be ~r from center
  r_end = sqrt((end.x - center.x)² + (end.y - center.y)²)
  if abs(r - r_end) > tolerance: warn("arc radii mismatch")

Angles:
  start_angle = atan2(start.y - center.y, start.x - center.x)
  end_angle   = atan2(end.y - center.y, end.x - center.x)

Sweep (G75 multi-quadrant mode):
  if direction == CW:
    if start_angle <= end_angle: sweep = start_angle - end_angle - 2π
    else:                        sweep = start_angle - end_angle
  if direction == CCW:
    if end_angle <= start_angle: sweep = end_angle - start_angle + 2π
    else:                        sweep = end_angle - start_angle

  // Full circle: if start == end and I,J != 0
  if start ≈ end and (I != 0 or J != 0):
    sweep = ±2π (direction-dependent)

Tessellation:
  N = max(MIN_SEGMENTS, ceil(abs(sweep) * r / max_segment_length))
  for i in 0..=N:
    angle = start_angle + sweep * (i / N)
    point = (center.x + r * cos(angle), center.y + r * sin(angle))
```

---

## 19. Service Worker Architecture

### 19.1 Strategy: Cache-First with Version Bumps

```
                    ┌──────────────┐
                    │   Browser    │
                    │   Request    │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐     HIT
                    │  Cache API   │─────────► Return cached response
                    └──────┬───────┘
                           │ MISS
                    ┌──────▼───────┐
                    │   Network    │─────────► Return + cache response
                    └──────────────┘
```

### 19.2 Versioned Cache

```javascript
// public/sw.js
const CACHE_VERSION = "gerberview-v1";  // Bump on each deploy

const PRECACHE_URLS = [
  "/",
  "/index.html",
  "/assets/main.[hash].js",
  "/assets/main.[hash].css",
  "/gerberview_wasm_bg.wasm",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_VERSION).then((cache) => cache.addAll(PRECACHE_URLS))
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((names) =>
      Promise.all(
        names
          .filter((name) => name !== CACHE_VERSION)
          .map((name) => caches.delete(name))
      )
    )
  );
  self.clients.claim();
});

self.addEventListener("fetch", (event) => {
  event.respondWith(
    caches.match(event.request).then((cached) => cached || fetch(event.request))
  );
});
```

### 19.3 WASM Module Caching

The `.wasm` file is cached by the service worker like any other static asset. `WebAssembly.compileStreaming()` is used for initial load (compiles while downloading). On subsequent loads, the compiled module is retrieved from cache.

---

## 20. Extension Points

These are places where future features can be added with minimal disruption to existing code.

| Future Feature | Extension Point | What Changes |
|---------------|----------------|-------------|
| Measurement tool (ruler) | Add `MeasurementOverlay` node to `OverlayGroup` | New file: `measurement-overlay.ts`. No renderer changes. |
| Grid overlay | Add `GridOverlay` node to `OverlayGroup` | New file: `grid-overlay.ts`. New shader for dashed lines. |
| Export to PNG | Read canvas pixels via `gl.readPixels()` | New module: `export.ts`. No architecture changes. |
| Gerber X2 metadata | Extend `LayerMeta` with optional metadata fields | Rust: parse X2 attributes. TS: display in layer panel. |
| Net highlighting | Add net data to `LayerNode`, filter vertices by net | Rust: track net assignments during parse. TS: new shader with per-vertex color. |
| 3D rendering | Replace WebGL 1.0 renderer with WebGL 2.0 / Three.js | New renderer module. Scene graph abstracts the transition. |
| Stencil-based polarity | Render clear polarity via stencil buffer | Modify `renderer.ts` draw loop only. |
| File format detection from content (not just filename) | Extend `layer-identify.ts` with content-sniffing | No architecture changes. |

The scene graph and reactive store were chosen specifically to make these extensions possible without rewriting the core.

---

## 21. Decision Log

| ID | Decision | Rationale | Alternatives Considered |
|----|----------|-----------|------------------------|
| AD-001 | Web Worker for parsing | UI responsiveness is architectural driver #2. Complex boards could block main thread for 1-2s. | Main-thread parsing (simpler but blocks UI), `OffscreenCanvas` (not needed — WASM has no DOM) |
| AD-002 | On-demand rendering (dirty flag) | Laptops, battery life, power efficiency. Zero GPU usage when view is static. | Continuous rAF loop (simpler, but wastes power) |
| AD-003 | Custom reactive signals (not a framework) | ~40 LOC. No framework dependency. Full type safety. Sufficient for 10 signals. | Preact Signals (adds dependency), RxJS (way too heavy), plain callbacks (no computed) |
| AD-004 | Scene graph abstraction | Enables overlays (measurement, grid) without touching renderer. Centralizes VBO lifecycle management. | Direct rendering (simpler but locks out future features) |
| AD-005 | `Vec<f32>` return (copy) over `Float32Array` view | Worker transfer requires owned buffer. View would need manual copy anyway. Equivalent performance, safer API. | Float32Array view (feasibility doc's original proposal, revised for Worker architecture) |
| AD-006 | `Transferable` over `SharedArrayBuffer` | No COOP/COEP header requirements. Simpler ownership model. Data transfer is one-shot, not streaming. | SharedArrayBuffer (requires special headers, complicates CF Pages) |
| AD-007 | Cooperative cancellation (request ID matching) | WASM parse is synchronous per file and cannot be interrupted. Layer-level cancellation is sufficient. | AbortController (doesn't work with synchronous WASM), Web Worker termination (loses WASM init) |
| AD-008 | pnpm (not npm or yarn) | Strict dependency resolution, faster installs, disk-efficient. Required by Turborepo. | npm (loose hoisting), yarn (less strict than pnpm) |
| AD-009 | Single shader program for MVP | Only flat-colored triangles needed. One program, two uniforms. | Per-layer shaders (unnecessary complexity), instanced rendering (WebGL 2.0 only) |
| AD-010 | Retain JS vertex buffers until context stable | WebGL context loss can happen. Need source data to re-upload VBOs. | Release immediately after upload (saves RAM but cannot recover from context loss) |

---

> **End of Architecture & Design Document**  
> **Next step:** Phase 0 — Project scaffolding.
