// API-level UI smoke test: plays puzzle #25 (turtle) move by move through
// the same wasm calls the app UI makes (state + can_remove), asserting the
// game reaches "won". Run: node scripts/play-solution.mjs
import { readFileSync } from "node:fs";
import init, { puzzle_for, state, can_remove } from "../wasm-core/mahjong_wasm.js";

const bytes = readFileSync(
  new URL("../wasm-core/mahjong_wasm_bg.wasm", import.meta.url),
);
await init(bytes);

const p = puzzle_for(25, "turtle");
let faces = Uint8Array.from(p.faces);
const total = p.faces.length;
let moves = 0;

const st0 = state("turtle", faces);
console.log(`start: ${st0.remaining} tiles, ${st0.free.length} free, ${st0.moves.length} legal moves`);
if (st0.remaining !== total) throw new Error(`expected ${total} tiles, got ${st0.remaining}`);
if (total % 2 !== 0) throw new Error("odd tile count");
if (st0.moves.length === 0) throw new Error("no legal moves at start");

for (const [a, b] of p.solution) {
  if (!can_remove("turtle", faces, a, b)) {
    throw new Error(`solution pair (${a},${b}) rejected at move ${moves + 1}`);
  }
  faces[a] = 255;
  faces[b] = 255;
  moves++;
  if (moves % 36 === 0) {
    const st = state("turtle", faces);
    console.log(`  after ${moves} pairs: ${st.remaining} left, stuck=${st.stuck}, won=${st.won}`);
  }
}

const end = state("turtle", faces);
if (!end.won) throw new Error("board not cleared after replaying full solution");
if (end.remaining !== 0) throw new Error(`remaining=${end.remaining}`);
console.log(`PASS: played all ${moves} pairs, board cleared (won=true)`);

// sanity: hint must exist while moves remain, and deadlock must report
const mid = state("turtle", Uint8Array.from(p.faces));
if (!mid.hint) throw new Error("no hint on fresh board");
console.log(`hint on fresh board: (${mid.hint[0]},${mid.hint[1]})`);
