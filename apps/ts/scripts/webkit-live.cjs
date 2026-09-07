// LIVE verification of both deployed apps in real WebKit at iPhone metrics.
const { webkit } = require("playwright");

(async () => {
  const browser = await webkit.launch();
  const ctx = await browser.newContext({
    viewport: { width: 390, height: 844 },
    hasTouch: true, isMobile: true, deviceScaleFactor: 3,
  });

  // --- TS app ---
  const ts = await ctx.newPage();
  const errs = [];
  ts.on("pageerror", (e) => errs.push(e.message));
  await ts.goto("https://danielachandler.github.io/mahjong/", { waitUntil: "networkidle" });
  await ts.waitForTimeout(2500);
  const tsBoot = await ts.evaluate(() => {
    const b = document.querySelector(".board-inner");
    const rect = b?.getBoundingClientRect();
    return {
      tiles: document.querySelectorAll(".tile").length,
      rectRight: Math.round(rect?.right ?? -1),
      rectBottom: Math.round(rect?.bottom ?? -1),
      scrollW: document.documentElement.scrollWidth,
      innerW: window.innerWidth,
      svgFaces: document.querySelectorAll(".tile-face svg").length,
    };
  });
  await ts.tap("#btn-menu");
  await ts.waitForTimeout(500);
  const tsMenu = await ts.evaluate(() => ({
    open: !!(document.querySelector("#menu-dialog")?.hasAttribute("open") ||
             document.querySelector("#menu-dialog")?.classList.contains("polyfill-open")),
    opts: document.querySelectorAll("#menu-dialog select option, #menu-dialog button").length,
  }));
  await ts.evaluate(() => document.querySelector("#btn-menu")?.click()); // leave open state
  await ts.evaluate(() => { const d = document.querySelector("#menu-dialog"); if (d) { d.close ? d.close() : d.removeAttribute("open"); d.classList.remove("polyfill-open"); } });
  console.log("LIVE-TS", JSON.stringify({ ...tsBoot, ...tsMenu, errors: errs.length }));
  await ts.screenshot({ path: process.env.LOCALAPPDATA + "/Temp/mj-live-webkit-ts.png" });

  // --- Rust app ---
  const ru = await ctx.newPage();
  const rerrs = [];
  ru.on("pageerror", (e) => rerrs.push(e.message));
  await ru.goto("https://danielachandler.github.io/mahjong/rust/", { waitUntil: "networkidle" });
  await ru.waitForTimeout(2500);
  const ruBoot = await ru.evaluate(() => ({
    tiles: document.querySelectorAll(".tile").length,
    svgFaces: document.querySelectorAll(".tile-face svg").length,
    scrollW: document.documentElement.scrollWidth,
    bodyText: (document.body.textContent || "").slice(0, 40),
  }));
  await ru.tap("#btn-menu");
  await ru.waitForTimeout(500);
  const ruMenu = await ru.evaluate(() => ({
    visible: !document.querySelector("#menu-overlay")?.hasAttribute("hidden"),
    layouts: document.querySelectorAll("#menu-layouts button").length,
    themes: document.querySelectorAll("#menu-themes button").length,
  }));
  console.log("LIVE-RUST", JSON.stringify({ ...ruBoot, ...ruMenu, errors: rerrs.length }));
  await ru.screenshot({ path: process.env.LOCALAPPDATA + "/Temp/mj-live-webkit-rust.png" });

  await browser.close();
  process.exit(0);
})().catch((e) => { console.error("ERR", e.message); process.exit(1); });
