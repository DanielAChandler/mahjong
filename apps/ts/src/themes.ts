// Theme registry. The DATA lives in /shared/themes/themes.json — defined
// once, consumed by both implementations (the Rust app includes the same
// file at compile time).

import shared from "../../../shared/themes/themes.json";

export interface ThemeTokens {
  id: string;
  name: string;
  palette: {
    boardBg: string;
    boardBg2: string;
    tileFace: string;
    tileEdge: string;
    tileSide: string;
    accent: string;
    text: string;
  };
  fontFamily: string;
  style: "classic" | "flat" | "seasonal";
}

export const THEMES: ThemeTokens[] = shared.themes as ThemeTokens[];
export const SKINS: { id: string; name: string; css: string }[] = shared.skins as any;

export function themeById(id: string): ThemeTokens {
  return THEMES.find((t) => t.id === id) ?? THEMES[0];
}
