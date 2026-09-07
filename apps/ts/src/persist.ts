// Persistence: settings + stats + coins, localStorage-backed, with
// export/import for cross-device moves (spec requirement).

import type { ThemeTokens } from "./themes.js";

const KEY = "mahjong-ts.v1";

export interface Settings {
  theme: string;
  skin: string;
  sound: boolean;
}

export interface Stats {
  puzzlesCompleted: number;
  bestTimes: Record<string, number>; // key `layout:pid` or `camp:level`
  currentStreak: number;
  bestStreak: number;
  lastPlayedISO: string | null;
  hintsUsed: number;
  shufflesUsed: number;
  undosUsed: number;
  dynamitesUsed: number;
  wins: number;
  losses: number;
}

export interface SaveState {
  coins: number;
  settings: Settings;
  stats: Stats;
  dailyDone: Record<string, true>; // yyyy-mm-dd -> done
}

export function defaultState(): SaveState {
  return {
    coins: 120,
    settings: { theme: "classic", skin: "felt", sound: true },
    stats: {
      puzzlesCompleted: 0,
      bestTimes: {},
      currentStreak: 0,
      bestStreak: 0,
      lastPlayedISO: null,
      hintsUsed: 0,
      shufflesUsed: 0,
      undosUsed: 0,
      dynamitesUsed: 0,
      wins: 0,
      losses: 0,
    },
    dailyDone: {},
  };
}

export function load(): SaveState {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return defaultState();
    const parsed = JSON.parse(raw) as Partial<SaveState>;
    const def = defaultState();
    return {
      coins: parsed.coins ?? def.coins,
      settings: { ...def.settings, ...(parsed.settings ?? {}) },
      stats: { ...def.stats, ...(parsed.stats ?? {}) },
      dailyDone: parsed.dailyDone ?? {},
    };
  } catch {
    return defaultState();
  }
}

export function save(s: SaveState): void {
  localStorage.setItem(KEY, JSON.stringify(s));
}

export function exportJson(s: SaveState): string {
  return JSON.stringify({ app: "mahjong-ts", version: 1, savedAt: new Date().toISOString(), state: s }, null, 2);
}

export function importJson(text: string): SaveState {
  const obj = JSON.parse(text);
  const st = obj.state ?? obj;
  const def = defaultState();
  return {
    coins: Number(st.coins) || def.coins,
    settings: { ...def.settings, ...(st.settings ?? {}) },
    stats: { ...def.stats, ...(st.stats ?? {}) },
    dailyDone: st.dailyDone ?? {},
  };
}

// ---- economy ----
export const COSTS = { hint: 20, shuffle: 40, undo: 15, dynamite: 60 };
export const REWARDS = { win: 30, streakBonus: 5, daily: 50 };

export function todayKey(): string {
  return new Date().toISOString().slice(0, 10);
}

export function applyWin(s: SaveState, key: string, seconds: number): SaveState {
  s.stats.puzzlesCompleted += 1;
  s.stats.wins += 1;
  s.stats.losses = Math.max(0, s.stats.losses);
  const prev = s.stats.bestTimes[key];
  if (prev === undefined || seconds < prev) s.stats.bestTimes[key] = seconds;
  s.coins += REWARDS.win;
  const today = todayKey();
  const y = new Date(Date.now() - 86400_000).toISOString().slice(0, 10);
  s.stats.currentStreak =
    s.stats.lastPlayedISO === y || s.stats.lastPlayedISO === today
      ? s.stats.currentStreak + 1
      : 1;
  s.stats.bestStreak = Math.max(s.stats.bestStreak, s.stats.currentStreak);
  s.stats.lastPlayedISO = today;
  if (!s.dailyDone[today]) {
    s.dailyDone[today] = true;
    s.coins += REWARDS.daily;
  }
  s.coins += Math.min(5, s.stats.currentStreak) * REWARDS.streakBonus;
  return s;
}
