// Engine: game session orchestration for the TS app. ALL rules come from
// the wasm core — this file never decides legality itself.

import * as api from "./api.js";
import type { Renderer, RenderTile } from "./renderer.js";
import { sfx } from "./sound.js";

export interface SlotDto {
  idx: number;
  x: number;
  y: number;
  z: number;
}

export interface PuzzleDto {
  puzzle_id: number;
  layout_id: string;
  faces: number[];
  solution: [number, number][];
  difficulty: {
    tile_count: number;
    max_height: number;
    distinct_groups: number;
    rating: number;
  };
}

export type Mode = { kind: "campaign"; level: number } | { kind: "infinite"; id: number; layoutId: string } | { kind: "daily" };

/** wasm bindgen wants a real Uint8Array for &[u8] params. */
function u8(a: number[]): Uint8Array {
  return Uint8Array.from(a);
}

export interface Hooks {
  onWin(seconds: number): void;
  onStuck(): void;
  onToast(msg: string): void;
  onHud(hud: { pairs: number; free: number }): void;
}

export class Game {
  private puzzle!: PuzzleDto;
  private faces: number[] = [];
  private faceIds = FACE_IDS;
  private selected: number | null = null;
  private undoStack: { faces: number[] }[] = [];
  private timer = 0;
  private tick: number | null = null;
  private hintPair: [number, number] | null = null;
  stuckNotified = false;

  constructor(
    private renderer: Renderer,
    private hooks: Hooks,
    private sound: boolean,
  ) {}

  async start(mode: Mode) {
    this.puzzle =
      mode.kind === "campaign"
        ? ((await api.campaign_puzzle(mode.level)) as PuzzleDto)
        : ((await api.puzzle_for(mode.kind === "daily" ? dailyPuzzleId() : mode.id, mode.kind === "daily" ? "turtle" : mode.layoutId)) as PuzzleDto);
    this.faces = [...this.puzzle.faces];
    this.selected = null;
    this.undoStack = [];
    this.timer = 0;
    this.stuckNotified = false;
    this.hintPair = null;

    // geometry for the renderer: derive real grid dims from the slot coords
    // (the board must scale to the ACTUAL layout extent, not a hardcoded grid)
    const slots = api.slot_coords(this.puzzle.layout_id);
    const maxX = Math.max(...slots.map((s: any) => s.x));
    const maxY = Math.max(...slots.map((s: any) => s.y));
    const maxZ = Math.max(...slots.map((s: any) => s.z));
    this.renderer.setGeometry({ w: maxX + 1, h: maxY + 1, maxZ: maxZ + 1 });

    const tiles = this.renderTiles();
    this.renderer.build(tiles, (idx) => this.onTile(idx));
    this.refresh();
    this.startTimer();
  }

  private renderTiles(): RenderTile[] {
    // slot coords come from the wasm catalog's compiled order — we rebuild
    // them from the puzzle faces length + layout JSON embedded in wasm.
    const slots = api.slot_coords(this.puzzle.layout_id);
    return this.faces.map((f, i) => ({
      idx: i,
      x: slots[i].x,
      y: slots[i].y,
      z: slots[i].z,
      face: f,
      faceId: f === 255 ? "" : this.faceIds[f],
      removed: f === 255,
    }));
  }

  private async refresh() {
    const st = await api.state(this.puzzle.layout_id, u8(this.faces));
    this.renderer.markFree(st.free);
    if (this.selected !== null) this.renderer.markSelected(this.selected);
    this.hooks.onHud({ pairs: st.remaining / 2, free: st.free.length });
    if (st.won) {
      this.stopTimer();
      this.sound && sfx.win();
      this.hooks.onWin(this.timer);
      return;
    }
    if (st.stuck && !this.stuckNotified) {
      this.stuckNotified = true;
      this.sound && sfx.fail();
      this.hooks.onStuck();
    }
    if (!st.stuck) this.stuckNotified = false;
    return st;
  }

  private async onTile(idx: number) {
    const st = await api.state(this.puzzle.layout_id, u8(this.faces));
    if (this.selected === null) {
      if (!st.free.includes(idx)) return;
      this.selected = idx;
      this.renderer.markSelected(idx);
      this.sound && sfx.select();
      return;
    }
    if (this.selected === idx) {
      this.selected = null;
      this.renderer.markSelected(null);
      return;
    }
    const ok = await api.can_remove(this.puzzle.layout_id, u8(this.faces), this.selected, idx);
    if (ok) {
      this.undoStack.push({ faces: [...this.faces] });
      const a = this.selected;
      this.faces[a] = 255;
      this.faces[idx] = 255;
      this.selected = null;
      this.renderer.markSelected(null);
      this.renderer.removePair(a, idx);
      this.sound && sfx.match();
    } else {
      this.sound && sfx.deny();
      // switch selection if the new tile is free
      if (st.free.includes(idx)) {
        this.selected = idx;
        this.renderer.markSelected(idx);
      }
    }
    await this.refresh();
  }

  async useHint() {
    const st = await api.state(this.puzzle.layout_id, u8(this.faces));
    if (!st.hint) {
      this.hooks.onToast("No moves available — shuffle!");
      return false;
    }
    this.hintPair = st.hint;
    this.renderer.markHint(this.hintPair);
    this.sound && sfx.power();
    setTimeout(() => this.renderer.markHint(null), 1600);
    return true;
  }

  async useShuffle() {
    const next = await api.shuffle(this.puzzle.layout_id, u8(this.faces), this.faces.reduce((a, f) => a + (f === 255 ? 1 : 0), 0) * 7919 + Date.now() % 7919);
    if (!next) {
      this.hooks.onToast("No reshuffle possible");
      return false;
    }
    this.undoStack.push({ faces: [...this.faces] });
    this.faces = next;
    this.renderer.rebuild(this.renderTiles(), (idx) => this.onTile(idx));
    this.sound && sfx.shuffle();
    await this.refresh();
    return true;
  }

  async useUndo() {
    const prev = this.undoStack.pop();
    if (!prev) {
      this.hooks.onToast("Nothing to undo");
      return false;
    }
    this.faces = prev.faces;
    this.selected = null;
    this.renderer.rebuild(this.renderTiles(), (idx) => this.onTile(idx));
    this.sound && sfx.power();
    await this.refresh();
    return true;
  }

  async useDynamite(idx: number | null) {
    // remove a single stuck tile: pick the selected tile, or a random free one
    let target = idx;
    if (target === null) {
      const st = await api.state(this.puzzle.layout_id, u8(this.faces));
      target = st.free[0];
    }
    if (target === null || target === undefined) return false;
    this.undoStack.push({ faces: [...this.faces] });
    this.faces[target] = 255;
    this.renderer.removePair(target, target);
    // odd face counts break solvability; also remove a random match partner
    const st = await api.state(this.puzzle.layout_id, u8(this.faces));
    const cand = st.moves[0];
    if (cand) {
      this.faces[cand[0]] = 255;
      this.faces[cand[1]] = 255;
      this.renderer.removePair(cand[0], cand[1]);
    }
    this.sound && sfx.power();
    await this.refresh();
    return true;
  }

  private startTimer() {
    this.stopTimer();
    this.tick = window.setInterval(() => this.timer++, 1000);
  }

  private stopTimer() {
    if (this.tick !== null) {
      clearInterval(this.tick);
      this.tick = null;
    }
  }

  elapsed(): number {
    return this.timer;
  }
}

export function dailyPuzzleId(): number {
  const d = new Date();
  return Number(`${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}`);
}

const FACE_IDS = [
  "dot1", "dot2", "dot3", "dot4", "dot5", "dot6", "dot7", "dot8", "dot9",
  "bam1", "bam2", "bam3", "bam4", "bam5", "bam6", "bam7", "bam8", "bam9",
  "chr1", "chr2", "chr3", "chr4", "chr5", "chr6", "chr7", "chr8", "chr9",
  "windE", "windS", "windW", "windN",
  "dragonR", "dragonG", "dragonW",
  "flower1", "flower2", "flower3", "flower4",
  "season1", "season2", "season3", "season4",
];
