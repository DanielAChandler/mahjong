// WebKit e2e for the Rust app: menu opens, layouts listed, theme switch works.
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

  await page.goto("http://localhost:4176/mahjong/rust/", { waitUntil: "networkidle" });
  await page.waitForTimeout(2500);

  const boot = await page.evaluate(() => ({
    tiles: document.querySelectorAll(".tile").length,
    boardW: document.querySelector(".board-inner")?.getBoundingClientRect().width,
    scrollW: document.documentElement.scrollWidth,
    innerW: window.innerWidth,
  }));
  console.log("BOOT", JSON.stringify(boot));

  // open menu
  await page.tap("#btn-menu");
  await page.waitForTimeout(500);
  const menu = await page.evaluate(() => {
    const ov = document.querySelector("#menu-overlay");
    return {
      visible: !!ov && !ov.hasAttribute("hidden"),
      layouts: document.querySelectorAll("#menu-layouts button").length,
      themes: document.querySelectorAll("#menu-themes button").length,
      panelH: document.querySelector("#menu-panel")?.getBoundingClientRect().height,
      innerH: window.innerHeight,
    };
  });
  console.log("MENU", JSON.stringify(menu));
  await page.screenshot({ path: process.env.LOCALAPPDATA + "/Temp/mj-webkit-rust-menu.png" });

  // switch theme to Midnight Garden (5th button: classic jade imperial flat midnight sakura)
  const themes = await page.$$("#menu-themes button");
  if (themes[4]) {
    await themes[4].tap();
    await page.waitForTimeout(800);
    const after = await page.evaluate(() => {
      const tile = document.querySelector(".tile");
      return {
        menuClosed: document.querySelector("#menu-overlay")?.hasAttribute("hidden"),
        faceSvg: !!tile?.querySelector(".tile-face svg"),
        tiles: document.querySelectorAll(".tile").length,
      };
    });
    console.log("THEME", JSON.stringify(after));
  }

  // pick a layout from menu
  await page.tap("#btn-menu");
  await page.waitForTimeout(400);
  const layouts = await page.$$("#menu-layouts button");
  if (layouts[2]) {
    const before = await page.evaluate(() => document.querySelectorAll(".tile").length);
    await layouts[2].tap();
    await page.waitForTimeout(800);
    const after = await page.evaluate((before) => ({
      menuClosed: document.querySelector("#menu-overlay")?.hasAttribute("hidden"),
      tiles: document.querySelectorAll(".tile").length,
      before,
    }), before);
    console.log("LAYOUT", JSON.stringify(after));
  }

  console.log("errors:", errors.length ? errors.join(" | ") : "(none)");
  await page.screenshot({ path: process.env.LOCALAPPDATA + "/Temp/mj-webkit-rust-after.png" });
  await browser.close();
  process.exit(0);
})().catch((e) => { console.error("ERR", e.message); process.exit(1); });
