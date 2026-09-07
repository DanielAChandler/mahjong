// Renderer: absolutely-positioned DOM tiles with CSS 3D-ish side faces.
// Chosen over canvas for crisp art at any DPI, easy hit-testing, and CSS
// transitions for lift/match animations.

import * as api from "./api.js";
import type { ThemeTokens } from "./themes.js";
import type { SlotDto } from "./engine.js";

export interface RenderTile {
  idx: number;
  x: number;
  y: number;
  z: number;
  face: number;
  faceId: string;
  removed: boolean;
}

export interface LayoutDims {
  w: number;
  h: number;
  maxZ: number;
}

export class Renderer {
  private root: HTMLElement;
  private boardEl: HTMLElement;
  private tiles = new Map<number, HTMLElement>();
  theme: ThemeTokens;
  dims: LayoutDims = { w: 10, h: 8, maxZ: 1 };

  // tuned tile metrics (px at scale 1). Tiles are PORTRAIT (taller than wide,
  // ~1:1.3 — the classic mahjong proportion) and laid out edge-to-edge (no
  // overlap) like Vita Mahjong; the board scales to fit.
  TW = 54;
  TH = 70;
  PX = 54; // horizontal pitch (== TW, tiles touch)
  PY = 70; // vertical pitch (== TH)
  ZX = 6; // small horizontal offset per z layer (stacked look)
  DZ = 9; // small vertical offset per z layer
  PAD = 24; // uniform padding around the centered content box
  private skinCss: string | null = null;

  constructor(root: HTMLElement, theme: ThemeTokens) {
    this.root = root;
    this.theme = theme;
    this.boardEl = document.createElement("div");
    this.boardEl.className = "board-inner";
    root.appendChild(this.boardEl);
    window.addEventListener("resize", () => this.fitToViewport());
    window.addEventListener("orientationchange", () => this.fitToViewport());
    window.visualViewport?.addEventListener("resize", () => this.fitToViewport());
    // iOS: host can be 0-sized at boot before flex settles; observe + retry.
    if ("ResizeObserver" in window) {
      new ResizeObserver(() => this.fitToViewport()).observe(root);
    }
    this.fitRetryLoop();
  }

  /** Re-apply fit until the host reports a non-zero size (max ~3s). */
  private fitRetryLoop(n = 0) {
    const host = this.boardEl.parentElement as HTMLElement | null;
    if (host && host.clientWidth > 0 && host.clientHeight > 0) {
      this.applyFit();
      return;
    }
    if (n > 40) return;
    requestAnimationFrame(() => setTimeout(() => this.fitRetryLoop(n + 1), 80));
  }

  setTheme(theme: ThemeTokens) {
    this.theme = theme;
    // re-render all faces on theme switch
    for (const [idx, el] of this.tiles) {
      el.dataset.faceId && this.paintFace(el);
    }
  }

  /** Board background skin (a fixed gradient) or null = theme gradient. */
  setSkin(css: string | null) {
    this.skinCss = css;
    this.paintBoard();
  }

  /** idx -> (x,y,z) mapping from the compiled layout (engine supplies). */
  setGeometry(dims: LayoutDims) {
    this.dims = dims;
  }

  clear() {
    this.boardEl.innerHTML = "";
    this.tiles.clear();
  }

  // build tile elements once per puzzle
  build(tiles: RenderTile[], onClick: (idx: number) => void) {
    this.clear();
    // Half-unit grid: tile coords are in half-units; full tile = 2×2 half-units.
    // Tile pixel pos = (x/2 * TW, y/2 * TH) + z lift. Content extent measured
    // from actual slot footprints, then centered in a padded box.
    let maxPX = 0, maxPY = 0;
    for (const t of tiles) {
      maxPX = Math.max(maxPX, (t.x / 2) * this.TW + this.TW + t.z * this.ZX);
      maxPY = Math.max(maxPY, (t.y / 2) * this.TH + this.TH + t.z * this.DZ);
    }
    this.boardEl.style.width = `${maxPX + this.PAD}px`;
    this.boardEl.style.height = `${maxPY + this.PAD}px`;

    for (const t of tiles) {
      if (t.removed) continue;
      const el = document.createElement("div");
      el.className = "tile";
      el.dataset.idx = String(t.idx);
      el.dataset.faceId = t.faceId;
      this.positionTile(el, t);
      this.paintFace(el);

      el.addEventListener("pointerdown", (e) => {
        e.preventDefault();
        onClick(t.idx);
      });
      this.boardEl.appendChild(el);
      this.tiles.set(t.idx, el);
    }
    this.paintBoard();
    this.fitToViewport();
  }

  /** Scale the fixed-size board down to fit the viewport (mobile-first). */
  fitToViewport() {
    // re-run after iOS orientation changes settle
    clearTimeout((this as any)._fitT);
    (this as any)._fitT = setTimeout(() => this.applyFit(), 120);
  }

  private applyFit() {
    const host = this.boardEl.parentElement; // #board
    if (!host) return;
    const availW = host.clientWidth - 8;
    const availH = host.clientHeight - 8;
    if (availW <= 0 || availH <= 0) return;
    const bw = this.boardEl.scrollWidth || parseFloat(this.boardEl.style.width) || 0;
    const bh = this.boardEl.scrollHeight || parseFloat(this.boardEl.style.height) || 0;
    if (!bw || !bh) return;
    const scale = Math.max(0.2, Math.min(1.6, availW / bw, availH / bh));
    this.boardEl.style.transform = `scale(${scale})`;
  }

  positionTile(el: HTMLElement, t: RenderTile) {
    // half-unit coords: full-tile steps are x%2==0 && y%2==0 (L0);
    // straddling layers have odd x/y offsets of half a tile.
    const px = (t.x / 2) * this.TW + t.z * this.ZX + this.PAD / 2;
    const py = (t.y / 2) * this.TH + t.z * this.DZ + this.PAD / 2;
    el.style.left = `${px}px`;
    el.style.top = `${py}px`;
    // stacking: z dominates, then row (y), then column (x)
    el.style.zIndex = String(t.z * 1000 + t.y * 32 + t.x);
  }

  paintFace(el: HTMLElement) {
    const t = this.theme;
    const faceId = el.dataset.faceId!;
    const svg = api.face_svg(faceId, t.id);
    el.innerHTML = `
      <div class="tile-side"></div>
      <div class="tile-face" style="background:${t.palette.tileFace}">
        <svg viewBox="0 0 100 130" width="100%" height="100%">${svg}</svg>
      </div>`;
    el.style.setProperty("--tile-edge", t.palette.tileEdge);
    el.style.setProperty("--tile-side", t.palette.tileSide);
  }

  markFree(idxs: number[]) {
    const set = new Set(idxs);
    for (const [idx, el] of this.tiles) {
      el.classList.toggle("free", set.has(idx));
    }
  }

  markSelected(idx: number | null) {
    for (const [i, el] of this.tiles) {
      el.classList.toggle("selected", i === idx);
    }
  }

  markHint(pair: [number, number] | null) {
    for (const [i, el] of this.tiles) {
      el.classList.toggle("hint", pair !== null && (i === pair[0] || i === pair[1]));
    }
  }

  removePair(a: number, b: number) {
    for (const idx of [a, b]) {
      const el = this.tiles.get(idx);
      if (el) {
        el.classList.add("matched");
        el.style.setProperty("--rx", `${(Math.random() * 40 - 20).toFixed(0)}px`);
        el.style.setProperty("--ry", `${(40 + Math.random() * 30).toFixed(0)}px`);
        setTimeout(() => el.remove(), 420);
        this.tiles.delete(idx);
      }
    }
  }

  /** Put removed tiles back (undo). Full rebuild is simplest + safe. */
  rebuild(tiles: RenderTile[], onClick: (idx: number) => void) {
    this.build(tiles, onClick);
  }

  private paintBoard() {
    const t = this.theme;
    const board = this.boardEl.parentElement!;
    if (this.skinCss) {
      board.style.background = this.skinCss;
    } else {
      // drive the CSS-gradient vars so the vignette overlay stays intact
      board.style.setProperty("--skin-a", t.palette.boardBg);
      board.style.setProperty("--skin-b", t.palette.boardBg2);
      board.style.background = "";
    }
  }
}
