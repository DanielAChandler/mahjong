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
  for (let round = 0; round < 200 && !cleared; round++) {
    const state = await page.evaluate(() => {
      const tiles = [...document.querySelectorAll(".tile")];
      if (tiles.length === 0) return "cleared";
      return { n: tiles.length };
    });
    if (state === "cleared") { cleared = true; break; }
    // query the engine's state directly (bypasses the hint power-up economy):
    // window.__game exposes the Game instance in dev/test builds
    let pair = await page.evaluate(async () => {
      const g = window.__game;
      if (!g) return null;
      const st = await g.apiState();
      return st.hint ? [String(st.hint[0]), String(st.hint[1])] : null;
    });
    if (!pair) {
      // ask the app for a hint (costs coins; the coin count is large enough
      // in tests) — always a legal pair including stacked ones
      await page.evaluate(() => {
        document.querySelector("#pu-hint")?.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, cancelable: true }));
      });
      await page.waitForTimeout(300);
      const hinted = await page.evaluate(() =>
        [...document.querySelectorAll(".tile.hint")].map((t) => t.dataset.idx));
      if (hinted.length < 2) {
        console.log("NO HINT at round", round, "tiles", state.n);
        break;
      }
      pair = [hinted[0], hinted[1]];
    }
    await page.evaluate((idx) => {
      document.querySelector(`[data-idx="${idx}"]`).dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, cancelable: true }));
    }, pair[0]);
    await page.waitForTimeout(100);
    await page.evaluate((idx) => {
      document.querySelector(`[data-idx="${idx}"]`).dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, cancelable: true }));
    }, pair[1]);
    await page.waitForTimeout(200);
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
