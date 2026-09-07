// UI wiring: menus, HUD, power-ups, settings, export/import, PWA.

import "./style.css";
import * as api from "./api.js";
import { Renderer } from "./renderer.js";
import { Game, dailyPuzzleId, type Mode } from "./engine.js";
import { THEMES, SKINS, themeById } from "./themes.js";
import * as persist from "./persist.js";
import { sfx } from "./sound.js";

const $ = <T extends HTMLElement>(sel: string): T => document.querySelector(sel)!;

// iOS < 15.4 has no <dialog>; emulate open/close + backdrop minimally.
function supportsDialog(): boolean {
  return (
    typeof HTMLDialogElement !== "undefined" &&
    typeof HTMLDialogElement.prototype.showModal === "function"
  );
}
function openDialog(dlg: HTMLElement) {
  if (supportsDialog()) {
    (dlg as HTMLDialogElement).showModal();
  } else {
    dlg.setAttribute("open", "");
    dlg.classList.add("polyfill-open");
  }
}
function closeDialog(dlg: HTMLElement) {
  if (supportsDialog()) (dlg as HTMLDialogElement).close();
  else dlg.removeAttribute("open");
}

let catalog: { layouts: { id: string; name: string; tile_count: number }[] } | null = null; // populated in boot
let save = persist.load();
let game: Game | null = null;
let renderer: Renderer;
let currentMode: Mode = { kind: "campaign", level: 1 };
let hintBudget = 5;

async function boot() {
  await api.init();
  console.info("mahjong core", api.mahjong_version());

  if ("serviceWorker" in navigator) {
    // relative registration → correct scope under the /mahjong/ Pages subpath
    navigator.serviceWorker.register("./sw.js").catch(() => {});
    // when a new SW activates and purges old caches, reload so we never run stale code
    let refreshing = false;
    navigator.serviceWorker.addEventListener("controllerchange", () => {
      if (refreshing) return;
      refreshing = true;
      location.reload();
    });
    navigator.serviceWorker.addEventListener("message", (e) => {
      if (e.data?.type === "SW_UPDATED" && !refreshing) {
        refreshing = true;
        location.reload();
      }
    });
  }

  catalog = await api.catalog();
  renderer = new Renderer($("#board"), themeById(save.settings.theme));
  game = new Game(renderer, {
    onWin: onWin,
    onStuck: () => toast("Deadlock — no moves left. Shuffle! 🔀"),
    onToast: toast,
    onHud: updateHud,
  }, save.settings.sound);

  bindUI();
  applyTheme();
  await game.start(currentMode);
}

function bindUI() {
  $("#btn-menu").addEventListener("click", openMenu);
  $("#pu-hint").addEventListener("click", () => spend("hint", async () => game!.useHint()));
  $("#pu-shuffle").addEventListener("click", () => spend("shuffle", async () => game!.useShuffle()));
  $("#pu-undo").addEventListener("click", () => spend("undo", async () => game!.useUndo()));
  $("#pu-dynamite").addEventListener("click", () => spend("dynamite", async () => game!.useDynamite(null)));

  window.addEventListener("keydown", (e) => {
    if (e.key === "h") $("#pu-hint").click();
    if (e.key === "s") $("#pu-shuffle").click();
    if (e.key === "u") $("#pu-undo").click();
    if (e.key === "d") $("#pu-dynamite").click();
    if (e.key === "Escape") game && game.useUndo();
  });

  document.addEventListener("pointerdown", () => sfx.select(), { once: true });
}

type PU = "hint" | "shuffle" | "undo" | "dynamite";
function spend(kind: PU, fn: () => Promise<boolean>) {
  const cost = persist.COSTS[kind];
  if (save.coins < cost) {
    toast(`Not enough coins for ${kind} (${cost})`);
    return;
  }
  fn().then((ok) => {
    if (ok) {
      save.coins -= cost;
      (save.stats as any)[`${kind === "dynamite" ? "dynamites" : kind + "s"}Used`] += 1;
      persist.save(save);
      updatePowerups();
    }
  });
}

function updatePowerups() {
  const el = (id: string) => $(id);
  el("#pu-hint .pu-count").textContent = String(Math.floor(save.coins / persist.COSTS.hint));
  el("#pu-shuffle .pu-count").textContent = String(Math.floor(save.coins / persist.COSTS.shuffle));
  el("#pu-undo .pu-count").textContent = String(Math.floor(save.coins / persist.COSTS.undo));
  el("#pu-dynamite .pu-count").textContent = String(Math.floor(save.coins / persist.COSTS.dynamite));
  $("#hud-level").textContent = modeLabel(currentMode);
}

let lastHud = { pairs: 0, free: 0 };

function updateHud(h: { pairs: number; free: number }) {
  lastHud = h;
  $("#hud-pairs").textContent = `${h.pairs} pairs`;
  const t = game?.elapsed() ?? 0;
  $("#hud-timer").textContent = `${Math.floor(t / 60)}:${String(t % 60).padStart(2, "0")}`;
}

function modeLabel(m: Mode): string {
  if (m.kind === "campaign") return `Level ${m.level}`;
  if (m.kind === "daily") return `Daily ${dailyPuzzleId()}`;
  return `Puzzle #${m.id}`;
}

async function onWin(seconds: number) {
  const key =
    currentMode.kind === "campaign"
      ? `camp:${currentMode.level}`
      : currentMode.kind === "daily"
        ? `daily:${dailyPuzzleId()}`
        : `${currentMode.layoutId}:${currentMode.id}`;
  persist.applyWin(save, key, seconds);
  persist.save(save);
  updatePowerups();
  const dlg = $("#win-dialog") as HTMLDialogElement;
  const t = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  dlg.innerHTML = `
    <h2>🎉 Board cleared!</h2>
    <section>
      <p id="win-stats">Time ${t} — +${persist.REWARDS.win} coins (streak ${save.stats.currentStreak})</p>
      <p>🏆 ${save.stats.puzzlesCompleted} cleared · best streak ${save.stats.bestStreak} · ${save.coins} coins</p>
      <button data-wact="next">▶ Next level</button>
      <button data-wact="replay">↻ Play again</button>
      <button data-wact="menu">☰ Menu</button>
    </section>`;
  dlg.onclick = (e) => {
    const el = (e.target as HTMLElement).closest("button");
    if (!el) return;
    closeDialog(dlg);
    if (el.dataset.wact === "next") {
      currentMode = { kind: "campaign", level: nextLevel() };
      game!.start(currentMode);
      updatePowerups();
    } else if (el.dataset.wact === "replay") {
      game!.start(currentMode);
      updatePowerups();
    } else openMenu();
  };
  openDialog(dlg);
}

function toast(msg: string) {
  const t = $("#toast");
  t.textContent = msg;
  t.hidden = false;
  clearTimeout((t as any)._h);
  (t as any)._h = setTimeout(() => (t.hidden = true), 2200);
}

function applyTheme() {
  document.body.dataset.theme = save.settings.theme;
  renderer?.setTheme(themeById(save.settings.theme));
  applySkin();
}

function skinCss(id: string): string | null {
  const s = SKINS.find((sk) => sk.id === id);
  return s && s.id !== "felt" ? s.css : null;
}

function applySkin() {
  renderer?.setSkin(skinCss(save.settings.skin));
}

function openMenu() {
  const dlg = $("#menu-dialog") as HTMLDialogElement;
  const credit = document.createElement("p");
  credit.className = "menu-credit";
  credit.textContent = "Tile art: 碧海风 (Bihai feng), CC BY-SA 4.0 (Wikimedia Commons)";
  dlg.innerHTML = `
    <h2>Mahjong Solitaire</h2>
    <section>
      <h3>Play</h3>
      <button data-act="next">▶ Continue campaign (Level ${nextLevel()})</button>
      <button data-act="daily">📅 Daily puzzle</button>
      <div style="display:flex;gap:8px">
        <input id="inp-puzzle" type="number" min="1" placeholder="puzzle #" />
        <button data-act="load">Load #</button>
      </div>
      <select id="sel-layout">${(catalog?.layouts ?? []).map((l) => `<option value="${l.id}">${l.name} (${l.tile_count})</option>`).join("")}</select>
    </section>
    <section>
      <h3>Theme</h3>
      <div class="theme-row">
        ${THEMES.map((t) => `<button data-theme="${t.id}" class="${save.settings.theme === t.id ? "active" : ""}">${t.name}</button>`).join("")}
      </div>
      <h3>Board skin</h3>
      <div class="theme-row">
        ${SKINS.map((s) => `<button data-skin="${s.id}" class="${save.settings.skin === s.id ? "active" : ""}">${s.name}</button>`).join("")}
      </div>
      <h3>Sound</h3>
      <button data-act="sound">🔊 Sound: ${save.settings.sound ? "On" : "Off"}</button>
    </section>
    <section>
      <h3>Progress</h3>
      <p>🏆 ${save.stats.puzzlesCompleted} cleared · best streak ${save.stats.bestStreak} · ${save.coins} coins</p>
      <button data-act="export">⬇ Export progress</button>
      <button data-act="import">⬆ Import progress</button>
      <button data-act="reset">Reset everything</button>
    </section>
    <button data-act="close">Close</button>`;
  dlg.appendChild(credit);
  openDialog(dlg);

  dlg.onclick = (e) => {
    const el = (e.target as HTMLElement).closest("button");
    if (!el) return;
    if (el.dataset.theme) {
      save.settings.theme = el.dataset.theme;
      persist.save(save);
      applyTheme();
      openMenu();
      return;
    }
    if (el.dataset.skin) {
      save.settings.skin = el.dataset.skin;
      persist.save(save);
      applySkin();
      openMenu();
      return;
    }
    switch (el.dataset.act) {
      case "sound":
        save.settings.sound = !save.settings.sound;
        persist.save(save);
        game?.setSound(save.settings.sound);
        openMenu();
        return;
      case "next":
        currentMode = { kind: "campaign", level: nextLevel() };
        closeDialog(dlg);
        game!.start(currentMode);
        updatePowerups();
        break;
      case "daily":
        currentMode = { kind: "daily" };
        closeDialog(dlg);
        game!.start(currentMode);
        updatePowerups();
        break;
      case "load": {
        const v = Number((dlg.querySelector("#inp-puzzle") as HTMLInputElement).value);
        const lay = (dlg.querySelector("#sel-layout") as HTMLSelectElement).value;
        if (v > 0) {
          currentMode = { kind: "infinite", id: v, layoutId: lay };
          closeDialog(dlg);
          game!.start(currentMode);
          updatePowerups();
        }
        break;
      }
      case "export": {
        const blob = new Blob([persist.exportJson(save)], { type: "application/json" });
        const a = document.createElement("a");
        a.href = URL.createObjectURL(blob);
        a.download = `mahjong-progress-${persist.todayKey()}.json`;
        a.click();
        break;
      }
      case "import": {
        const inp = document.createElement("input");
        inp.type = "file";
        inp.accept = "application/json";
        inp.onchange = async () => {
          const f = inp.files?.[0];
          if (!f) return;
          try {
            save = persist.importJson(await f.text());
            persist.save(save);
            applyTheme();
            updatePowerups();
            toast("Progress imported ✓");
            closeDialog(dlg);
          } catch {
            toast("Invalid save file");
          }
        };
        inp.click();
        break;
      }
      case "reset":
        save = persist.defaultState();
        persist.save(save);
        applyTheme();
        updatePowerups();
        toast("Progress reset");
        closeDialog(dlg);
        break;
      case "close":
        closeDialog(dlg);
        break;
    }
  };
}

function nextLevel(): number {
  const done = save.stats.puzzlesCompleted;
  return Math.min(300, (done % 300) + 1);
}

// timer repaint (keeps last known pair count)
setInterval(() => {
  if (game) updateHud(lastHud);
}, 1000);

boot();
