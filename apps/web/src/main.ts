import "./styles/main.css";
import init, { ping } from "gerberview-wasm";

async function main(): Promise<void> {
  await init();

  const result = ping();
  const statusEl = document.getElementById("status-text");
  if (statusEl) {
    statusEl.textContent = `WASM ready (ping: ${String(result)})`;
  }
}

void main();
