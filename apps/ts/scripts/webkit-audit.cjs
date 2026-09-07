// Per-layout fit audit in real WebKit at iPhone metrics.
// Iterates every layout in both apps; reports board rect, scrollWidth, overflow.
const { webkit } = require("playwright");

async function measure(page) {
  return page.evaluate(() => {
    const b = document.querySelector(".board-inner");
    const rect = b?.getBoundingClientRect();
    return {
      tiles: document.querySelectorAll(".tile").length,
      rectLeft: Math.round(rect?.left ?? -1),
      rectRight: Math.round(rect?.right ?? -1),
      rectTop: Math.round(rect?.top ?? -1),
      rectBottom: Math.round(rect?.bottom ?? -1),
      scrollW: document.documentElement.scrollWidth,
      innerW: window.innerWidth,
      innerH: window.innerHeight,
    };
  });
}

const bad = [];
(async () => {
  const browser = await webkit.launch();
  const ctx = await browser.newContext({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true, deviceScaleFactor: 3 });

  // ---------- TS ----------
  const ts = await ctx.newPage();
  ts.on("pageerror", (e) => console.log("TS PAGEERROR", e.message));
  await ts.goto("http://localhost:4176/mahjong/", { waitUntil: "networkidle" });
  await ts.waitForTimeout(2500);

  await ts.tap("#btn-menu");
  await ts.waitForTimeout(400);
  const layoutIds = await ts.evaluate(() =>
    [...document.querySelectorAll("#sel-layout option")].map((o) => o.value));
  console.log("TS layouts:", layoutIds.length, layoutIds.join(","));
  await ts.evaluate(() => {
    const d = document.querySelector("#menu-dialog");
    if (d?.hasAttribute("open")) d.close();
    d?.classList.remove("polyfill-open");
  });
  await ts.waitForTimeout(300);

  for (const id of layoutIds) {
    // open menu, set layout, load puzzle #1, close
    await ts.tap("#btn-menu");
    await ts.waitForTimeout(300);
    await ts.evaluate((id) => {
      const sel = document.querySelector("#sel-layout");
      sel.value = id;
      document.querySelector("#inp-puzzle").value = "1";
      document.querySelector('[data-act="load"]').click();
    }, id);
    await ts.waitForTimeout(1400);
    const m = await measure(ts);
    const overflow = m.rectRight > 391 || m.scrollW > 391 || m.rectLeft < -1 || m.rectBottom > 844;
    console.log(`TS ${id.padEnd(12)} tiles=${m.tiles} L=${m.rectLeft} R=${m.rectRight} B=${m.rectBottom} scrollW=${m.scrollW}${overflow ? "  <-- OVERFLOW" : ""}`);
    if (overflow) {
      bad.push(`ts:${id}`);
      await ts.screenshot({ path: `${process.env.LOCALAPPDATA}/Temp/audit-ts-${id}.png` });
    }
  }

  // ---------- Rust ----------
  const ru = await ctx.newPage();
  ru.on("pageerror", (e) => console.log("RUST PAGEERROR", e.message));
  await ru.goto("http://localhost:4176/mahjong/rust/", { waitUntil: "networkidle" });
  await ru.waitForTimeout(2500);

  await ru.tap("#btn-menu");
  await ru.waitForTimeout(400);
  const rustIds = await ru.evaluate(() =>
    [...document.querySelectorAll("#menu-layouts button")].map((b) => b.dataset.layout));
  console.log("\nRUST layouts:", rustIds.length, rustIds.join(","));

  for (let i = 0; i < rustIds.length; i++) {
    const id = rustIds[i];
    await ru.evaluate(() => document.querySelector("#menu-overlay")?.setAttribute("hidden", ""));
    await ru.waitForTimeout(250);
    await ru.tap("#btn-menu");
    await ru.waitForTimeout(300);
    const btns = await ru.$$("#menu-layouts button");
    if (btns[i]) await btns[i].tap();
    await ru.waitForTimeout(1400);
    const m = await measure(ru);
    const overflow = m.rectRight > 391 || m.scrollW > 391 || m.rectLeft < -1 || m.rectBottom > 844;
    console.log(`RUST ${id.padEnd(12)} tiles=${m.tiles} L=${m.rectLeft} R=${m.rectRight} B=${m.rectBottom} scrollW=${m.scrollW}${overflow ? "  <-- OVERFLOW" : ""}`);
    if (overflow) {
      bad.push(`rust:${id}`);
      await ru.screenshot({ path: `${process.env.LOCALAPPDATA}/Temp/audit-rust-${id}.png` });
    }
  }

  console.log("\nOVERFLOWING:", bad.length ? bad.join(", ") : "none");
  await browser.close();
  process.exit(0);
})().catch((e) => { console.error("ERR", e.message); process.exit(1); });
