// Deterministic reproduction of the deployed-app boot in Node, exercising
// the REAL bundle: fetch the live deployed index.html + JS bundle, run it
// against a simulated DOM (happy-dom), serve the wasm locally.
import { Window } from "happy-dom";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

const BASE = "https://danielachandler.github.io/mahjong/";
const html = await (await fetch(BASE)).text();

const assetFromHtml = html.match(/src="(\/mahjong\/assets\/index-[^"]+\.js)"/)?.[1];
if (!assetFromHtml) throw new Error("bundle not found in deployed html");
const bundleUrl = "https://danielachandler.github.io" + assetFromHtml;
const js = await (await fetch(bundleUrl)).text();
mkdirSync("tmp-diag", { recursive: true });
writeFileSync("tmp-diag/bundle.js", js);

const window = new Window({ url: BASE });
const { document } = window;
const errors = [];
window.addEventListener("error", (e) => errors.push("window.onerror: " + (e.error?.stack || e.message)));
window.addEventListener("unhandledrejection", (e) => errors.push("unhandledrejection: " + (e.reason?.stack || e.reason)));

// globals for the bundle
globalThis.window = window;
globalThis.document = document;
Object.defineProperty(globalThis, "navigator", { value: window.navigator, configurable: true });
globalThis.localStorage = window.localStorage;
globalThis.location = new URL(BASE);
globalThis.HTMLDialogElement = window.HTMLDialogElement;

// Node can't fetch file:// — serve wasm locally, rest passes through.
const realFetch = globalThis.fetch.bind(globalThis);
globalThis.fetch = async (input, initReq) => {
  const url = String(input?.url ?? input);
  if (url.endsWith(".wasm")) {
    const bytes = readFileSync(new URL("../wasm-core/mahjong_wasm_bg.wasm", import.meta.url));
    return new Response(bytes, { headers: { "content-type": "application/wasm" } });
  }
  return realFetch(input, initReq);
};

try {
  await import("../tmp-diag/bundle.js");
} catch (e) {
  errors.push("bundle import: " + (e.stack || e));
}

await new Promise((r) => setTimeout(r, 3000));
console.log("TILES:", document.querySelectorAll(".tile").length);
console.log("FREE:", document.querySelectorAll(".tile.free").length);
console.log("HUD:", document.getElementById("hud-level")?.textContent, "|", document.getElementById("hud-pairs")?.textContent);
console.log("ERRORS:", errors.length ? "" : "(none)");
errors.forEach((e) => console.log(" -", e.split("\n").slice(0, 5).join("\n   ")));
process.exit(0);
