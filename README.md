# GerberView

**A fast, free, browser-native Gerber PCB viewer. No signup, no upload, no backend.**

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

This project is in the early stages of development. See `docs/` for planning and design documents.

## License

[MIT](LICENSE)
