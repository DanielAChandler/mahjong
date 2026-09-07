// WebKit (real Safari engine) end-to-end: auto-play to win, verify dialog.
const { webkit } = require("playwright");

(async () => {
  const browser = await webkit.launch();
  const ctx = await browser.newContext({
    viewport: { width: 390, height: 844 },
    hasTouch: true, isMobile: true, deviceScaleFactor: 3,
  });
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });

  await page.goto("http://localhost:4176/mahjong/", { waitUntil: "networkidle" });
  await page.waitForTimeout(2000);

  let cleared = false;
  for (let round = 0; round < 40 && !cleared; round++) {
    const done = await page.evaluate(() => {
      const tiles = [...document.querySelectorAll(".tile")];
      if (tiles.length === 0) return "cleared";
      const free = [...document.querySelectorAll(".tile.free")];
      for (let i = 0; i < free.length; i++) {
        for (let j = i + 1; j < free.length; j++) {
          if (free[i].dataset.faceId === free[j].dataset.faceId) {
            return { a: free[i].dataset.idx, b: free[j].dataset.idx };
          }
        }
      }
      return "stuck";
    });
    if (done === "cleared") { cleared = true; break; }
    if (done === "stuck") { console.log("STUCK at round", round); break; }
    // dispatch pointerdown directly (overlapping tiles make coordinate taps ambiguous)
    await page.evaluate((idx) => {
      document.querySelector(`[data-idx="${idx}"]`).dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, cancelable: true }));
    }, done.a);
    await page.waitForTimeout(100);
    await page.evaluate((idx) => {
      document.querySelector(`[data-idx="${idx}"]`).dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, cancelable: true }));
    }, done.b);
    await page.waitForTimeout(480);
  }

  await page.waitForTimeout(1000);
  const m = await page.evaluate((cleared) => {
    const dlg = document.querySelector("#win-dialog");
    return {
      cleared,
      tilesLeft: document.querySelectorAll(".tile").length,
      winOpen: !!(dlg && (dlg.hasAttribute("open") || dlg.classList.contains("polyfill-open"))),
      winText: (dlg?.textContent || "").replace(/\s+/g, " ").slice(0, 90),
    };
  }, cleared);
  console.log(JSON.stringify(m));
  await page.screenshot({ path: process.env.LOCALAPPDATA + "/Temp/mj-webkit-win.png" });
  console.log("errors:", errors.length ? errors.join(" | ") : "(none)");
  await browser.close();
  process.exit(0);
})().catch((e) => { console.error("ERR", e.message); process.exit(1); });
