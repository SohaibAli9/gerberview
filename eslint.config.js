import { createConfig } from "@gerberview/eslint-config";

export default createConfig({
  tsconfigRootDir: import.meta.dirname,
  ignores: [
    "dist/",
    "e2e/",
    ".vite/",
    "*.config.*",
    "crates/",
    "docs/",
    "packages/eslint-config/",
  ],
});
