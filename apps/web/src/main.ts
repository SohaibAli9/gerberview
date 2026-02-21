import "./styles/main.css";
import init, { ping } from "gerberview-wasm";

async function main(): Promise<void> {
  await init();
  const result = ping();
  console.log("WASM ping:", result);
}

void main();
