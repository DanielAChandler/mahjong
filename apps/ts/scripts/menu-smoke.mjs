// Menu smoke test: boot the real built bundle in happy-dom, click the
// hamburger, assert the menu dialog opens with layout options populated.
import { Window } from "happy-dom";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

// happy-dom polyfills/globals the bundle expects
globalThis.ResizeObserver = class {
  observe() {}
  unobserve() {}
  disconnect() {}
};
globalThis.requestAnimationFrame = (cb) => setTimeout(() => cb(performance.now()), 16);
const { readdirSync } = await import("node:fs");
const assets = readdirSync("dist/assets");
const bundleName = assets.find((a) => a.startsWith("index-") && a.endsWith(".js"));
const wasmName = assets.find((a) => a.endsWith(".wasm"));
const jsCode = readFileSync(`dist/assets/${bundleName}`, "utf8");
mkdirSync("tmp-diag", { recursive: true });
writeFileSync("tmp-diag/bundle.js", jsCode);

const window = new Window({ url: "https://x.github.io/mahjong/" });
const { document } = window;
const errors = [];
window.addEventListener("error", (e) => errors.push(e.error?.stack || e.message));
window.addEventListener("unhandledrejection", (e) => errors.push(String(e.reason?.stack || e.reason)));

globalThis.window = window;
globalThis.document = document;
Object.defineProperty(globalThis, "navigator", { value: window.navigator, configurable: true });
globalThis.localStorage = window.localStorage;
globalThis.location = new URL("https://x.github.io/mahjong/");
globalThis.HTMLDialogElement = window.HTMLDialogElement;

const realFetch = globalThis.fetch.bind(globalThis);
globalThis.fetch = async (input, init) => {
  const url = String(input?.url ?? input);
  if (url.endsWith(".wasm")) {
    return new Response(readFileSync(`dist/assets/${wasmName}`), {
      headers: { "content-type": "application/wasm" },
    });
  }
  return realFetch(input, init);
};

document.body.innerHTML = readFileSync("index.html", "utf8").match(/<body>([\s\S]*)<\/body>/)[1];

try {
  await import("../tmp-diag/bundle.js");
} catch (e) {
  errors.push("import: " + (e.stack || e));
}
await new Promise((r) => setTimeout(r, 2500));

const tiles = document.querySelectorAll(".tile").length;
const btn = document.getElementById("btn-menu");
btn.dispatchEvent(new window.Event("click", { bubbles: true }));
await new Promise((r) => setTimeout(r, 200));

const dlg = document.getElementById("menu-dialog");
const open = dlg.hasAttribute("open") || dlg.classList.contains("polyfill-open");
const options = dlg.querySelectorAll("#sel-layout option").length;

console.log("TILES:", tiles);
console.log("MENU_OPEN:", open);
console.log("LAYOUT_OPTIONS:", options);
console.log("ERRORS:", errors.length ? errors.map((e) => e.split("\n")[0]).join(" ;; ") : "(none)");
process.exit(open && options >= 10 && tiles > 0 ? 0 : 1);
