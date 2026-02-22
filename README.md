# GerberView

**A fast, free, browser-native Gerber PCB viewer. No signup, no upload, no backend.**

[![CI](https://github.com/SohaibAli9/gerberview/actions/workflows/ci.yml/badge.svg)](https://github.com/SohaibAli9/gerberview/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white)
![WebGL](https://img.shields.io/badge/WebGL-990000?logo=webgl&logoColor=white)
![Tailwind CSS](https://img.shields.io/badge/Tailwind_CSS-06B6D4?logo=tailwindcss&logoColor=white)

> **Work in Progress** — This project is under active development.

---

## About

GerberView is a static web application that lets you inspect Gerber (RS-274X) and Excellon drill files directly in your browser. Drop a `.zip` of your board files and instantly see every layer rendered with correct colors, zoom, and pan — all powered by Rust/WASM parsing and WebGL rendering. Nothing ever leaves your machine.

## Tech Stack

- **Rust / WebAssembly** — Gerber & Excellon parsing, geometry conversion
- **TypeScript** — UI, file handling, interaction logic
- **WebGL 1.0** — GPU-accelerated 2D rendering at 60 fps
- **Tailwind CSS** — Dark-themed, minimal UI
- **Vite** — Frontend build tooling with WASM integration
- **Cloudflare Pages** — Static hosting, zero cost

## Status

**Phase 0 — Project Foundation: Complete**

| Task | Description                                     | Status |
| ---- | ----------------------------------------------- | ------ |
| T-00 | Git repository, README, LICENSE, .gitignore     | Done   |
| T-01 | Rust crate scaffold + WASM build verification   | Done   |
| T-02 | Web app scaffold (Vite + TypeScript + Tailwind) | Done   |
| T-03 | Monorepo tooling + code quality gates           | Done   |
| T-04 | CI pipeline (GitHub Actions)                    | Done   |
| T-05 | Test fixture files (Gerber, Excellon, ZIPs)     | Done   |

**Phase 1 — Core Types & Infrastructure: Complete**

| Task | Description                                     | Status |
| ---- | ----------------------------------------------- | ------ |
| T-06 | Rust core types, error types, GeometryBuilder   | Done   |
| T-07 | TypeScript types, constants, signals, app store | Done   |
| T-08 | Layer identification module                     | Done   |
| T-09 | ZIP handler module                              | Done   |

**Phase 2 — Parsing Pipeline: Complete**

| Task | Description                                    | Status |
| ---- | ---------------------------------------------- | ------ |
| T-10 | WASM bridge (`lib.rs`) + `parse_gerber` export | Done   |
| T-11 | Excellon drill parser                          | Done   |
| T-12 | Web Worker + `WorkerClient`                    | Done   |

**Phase 3 — Geometry Engine: Complete**

| Task | Description                                         | Status |
| ---- | --------------------------------------------------- | ------ |
| T-13 | Aperture expansion (circle, rect, obround, polygon) | Done   |
| T-14 | Stroke widening (D01 linear draw)                   | Done   |
| T-15 | Arc tessellation (G02/G03)                          | Done   |
| T-16 | Region fill (G36/G37)                               | Done   |
| T-17 | Polarity, step-repeat, aperture macros              | Done   |
| T-18 | Geometry converter orchestrator                     | Done   |
| T-19 | Rust integration tests + benchmarks                 | Done   |

**Phase 4 — WebGL Rendering: Complete**

| Task | Description                                      | Status |
| ---- | ------------------------------------------------ | ------ |
| T-20 | Shader compilation + WebGL context setup         | Done   |
| T-21 | Renderer module (draw loop, view matrix, layers) | Done   |
| T-22 | Scene graph manager (layer nodes, GPU buffers)   | Done   |
| T-23 | Interaction dispatcher (zoom, pan, keyboard)     | Done   |
| T-24 | Touch gestures (pinch-to-zoom, drag-to-pan)      | Done   |

Phase 5 (UI) is next. See `docs/` for planning and design documents.

## License

[MIT](LICENSE)
