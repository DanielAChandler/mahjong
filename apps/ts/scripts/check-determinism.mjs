// Cross-implementation determinism check (Node):
// generates puzzle #25 through the wasm module (the TS app's path) and
// hashes canonically — must equal the native core's hashes.
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import init, { puzzle_for } from "../wasm-core/mahjong_wasm.js";

const bytes = readFileSync(
  new URL("../wasm-core/mahjong_wasm_bg.wasm", import.meta.url),
);
await init(bytes);

for (const id of [1, 25, 999, 20260906]) {
  const p = puzzle_for(id, "turtle");
  let canon = p.faces.map((f) => f.toString(16).padStart(2, "0")).join("");
  canon += "|";
  for (const [a, b] of p.solution) canon += `${a}:${b},`;
  const hash = createHash("sha256").update(canon).digest("hex");
  console.log(`puzzle ${id}: ${hash}`);
}
