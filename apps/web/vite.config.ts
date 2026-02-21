import { defineConfig } from "vite";
import wasmPack from "vite-plugin-wasm-pack";
import topLevelAwait from "vite-plugin-top-level-await";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [
    wasmPack("../../crates/gerberview-wasm"),
    topLevelAwait(),
    tailwindcss(),
  ],
});
