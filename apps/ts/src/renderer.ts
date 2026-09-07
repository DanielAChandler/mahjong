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

  // tuned tile metrics (px at scale 1)
  TW = 56;
  TH = 72;
  DZ = 14; // visual z offset (isometric-ish)

  constructor(root: HTMLElement, theme: ThemeTokens) {
    this.root = root;
    this.theme = theme;
    this.boardEl = document.createElement("div");
    this.boardEl.className = "board-inner";
    root.appendChild(this.boardEl);
  }

  setTheme(theme: ThemeTokens) {
    this.theme = theme;
    // re-render all faces on theme switch
    for (const [idx, el] of this.tiles) {
      el.dataset.faceId && this.paintFace(el);
    }
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
    const pad = this.TW; // room for shadows/side
    this.boardEl.style.width = `${(this.dims.w + 1) * this.TW + pad}px`;
    this.boardEl.style.height = `${(this.dims.h + 1) * this.TH + this.DZ * (this.dims.maxZ + 1) + pad}px`;

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
    const host = this.boardEl.parentElement; // #board
    if (!host) return;
    const availW = host.clientWidth - 8;
    const availH = host.clientHeight - 8;
    if (availW <= 0 || availH <= 0) return;
    const bw = this.boardEl.scrollWidth || parseFloat(this.boardEl.style.width) || 0;
    const bh = this.boardEl.scrollHeight || parseFloat(this.boardEl.style.height) || 0;
    if (!bw || !bh) return;
    const scale = Math.min(1.6, availW / bw, availH / bh);
    this.boardEl.style.transform = `scale(${scale})`;
  }

  positionTile(el: HTMLElement, t: RenderTile) {
    const px = t.x * this.TW + t.z * 10;
    const py = t.y * this.TH + (this.dims.maxZ - t.z) * this.DZ;
    el.style.left = `${px}px`;
    el.style.top = `${py}px`;
    el.style.zIndex = String(t.z * 100 + t.y * 2);
  }

  paintFace(el: HTMLElement) {
    const t = this.theme;
    const idx = Number(el.dataset.idx);
    const faceId = el.dataset.faceId!;
    const svg = api.face_svg(faceId, t.id);
    el.innerHTML = `
      <div class="tile-side"></div>
      <div class="tile-face" style="background:${t.palette.tileFace}">
        <svg viewBox="0 0 60 60" width="100%" height="100%">${svg}</svg>
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
    this.boardEl.parentElement!.style.background = `radial-gradient(ellipse at center, ${t.palette.boardBg} 0%, ${t.palette.boardBg2} 75%)`;
  }
}
