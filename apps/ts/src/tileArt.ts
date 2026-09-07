// Tile face rendering: a single SVG generator parameterized by theme.
// Face ids come from mahjong-core (tiles.rs) — the stable spec:
// dot1..dot9, bam1..bam9, chr1..chr9, windE/S/W/N, dragonR/G/W,
// flower1..4, season1..4.

const D = {
  dot: { fill: "#2f6db3", ring: "#245487" },
  bam: { fill: "#2e8b57", ring: "#1f6641" },
  chr: { fill: "#b3402e", ring: "#8a2f22" },
  wind: { fill: "#4a4a6a", ring: "#33334d" },
  dragonG: "#2e8b57",
  dragonR: "#b3402e",
  dragonW: "#2f6db3",
};

export function faceSvg(
  face: number,
  faceId: string,
  theme: { palette: { text: string; accent: string }; style: string },
  size: number,
): string {
  const fg = theme.palette.text;
  const ac = theme.palette.accent;
  const flat = theme.style === "flat";
  const stroke = flat ? "none" : `stroke="${ac}" stroke-width="0.6"`;
  switch (true) {
    case faceId.startsWith("dot"):
      return dots(+faceId.slice(3), fg, size, flat);
    case faceId.startsWith("bam"):
      return bams(+faceId.slice(3), fg, size, flat);
    case faceId.startsWith("chr"):
      return chars(+faceId.slice(3), fg, size, flat);
    case faceId.startsWith("wind"):
      return windGlyph(faceId[4], fg, size);
    case faceId.startsWith("dragon"): {
      const c = faceId.endsWith("R")
        ? D.dragonR
        : faceId.endsWith("G")
          ? D.dragonG
          : D.dragonW;
      const label = faceId.endsWith("R")
        ? "中"
        : faceId.endsWith("G")
          ? "發"
          : "白";
      return `<text x="30" y="42" font-size="34" fill="${c}" text-anchor="middle" font-family="serif">${label}</text>`;
    }
    case faceId.startsWith("flower"):
      return flower(+faceId.slice(6), ac, size);
    case faceId.startsWith("season"):
      return season(+faceId.slice(6), ac, size);
    default:
      return `<text x="30" y="40" font-size="24" fill="${fg}">${face}</text>`;
  }
}

function dots(n: number, fg: string, size: number, flat: boolean): string {
  const pos: Record<number, [number, number][]> = {
    1: [[30, 30]],
    2: [
      [30, 16],
      [30, 44],
    ],
    3: [
      [16, 14],
      [30, 30],
      [44, 46],
    ],
    4: [
      [17, 17],
      [43, 17],
      [17, 43],
      [43, 43],
    ],
    5: [
      [17, 17],
      [43, 17],
      [30, 30],
      [17, 43],
      [43, 43],
    ],
    6: [
      [17, 14],
      [43, 14],
      [17, 30],
      [43, 30],
      [17, 46],
      [43, 46],
    ],
    7: [
      [17, 12],
      [43, 12],
      [17, 27],
      [43, 27],
      [17, 42],
      [43, 42],
      [30, 51],
    ],
    8: [
      [17, 12],
      [43, 12],
      [17, 26],
      [43, 26],
      [17, 40],
      [43, 40],
      [17, 52],
      [43, 52],
    ],
    9: [
      [15, 12],
      [30, 12],
      [45, 12],
      [15, 30],
      [30, 30],
      [45, 30],
      [15, 48],
      [30, 48],
      [45, 48],
    ],
  };
  const pts = pos[n] ?? pos[1];
  return pts
    .map(
      ([x, y]) =>
        `<circle cx="${x}" cy="${y}" r="6.5" fill="${D.dot.fill}" stroke="#ffffff" stroke-width="1.4"/><circle cx="${x}" cy="${y}" r="2.4" fill="#ffffff" opacity="0.85"/>`,
    )
    .join("");
}

const BAM_C = D.bam.fill;
function bams(n: number, fg: string, size: number, flat: boolean): string {
  const pos: Record<number, [number, number][]> = {
    1: [[30, 30]],
    2: [
      [22, 18],
      [38, 42],
    ],
    3: [
      [20, 14],
      [30, 30],
      [40, 46],
    ],
    4: [
      [20, 16],
      [40, 16],
      [20, 44],
      [40, 44],
    ],
    5: [
      [20, 16],
      [40, 16],
      [30, 30],
      [20, 44],
      [40, 44],
    ],
    6: [
      [20, 14],
      [40, 14],
      [20, 30],
      [40, 30],
      [20, 46],
      [40, 46],
    ],
    7: [
      [30, 12],
      [20, 27],
      [40, 27],
      [20, 42],
      [40, 42],
      [20, 54],
      [40, 54],
    ],
    8: [
      [20, 12],
      [40, 12],
      [20, 25],
      [40, 25],
      [20, 38],
      [40, 38],
      [20, 51],
      [40, 51],
    ],
    9: [
      [20, 12],
      [30, 12],
      [40, 12],
      [20, 30],
      [30, 30],
      [40, 30],
      [20, 48],
      [30, 48],
      [40, 48],
    ],
  };
  const pts = pos[n] ?? pos[1];
  return pts
    .map(
      ([x, y]) =>
        `<g transform="translate(${x},${y})"><rect x="-2.2" y="-7" width="4.4" height="14" rx="2" fill="${BAM_C}"/><rect x="-2.2" y="-7" width="1.6" height="14" rx="0.8" fill="#57b585" opacity="0.7"/></g>`,
    )
    .join("");
}

// Chinese numerals for character suit
const CHRN = ["一", "二", "三", "四", "五", "六", "七", "八", "九"];
function chars(n: number, fg: string, size: number, flat: boolean): string {
  const num = CHRN[n - 1] ?? "?";
  return `
    <text x="30" y="24" font-size="19" fill="${D.chr.fill}" text-anchor="middle" font-family="serif" font-weight="bold">${num}</text>
    <text x="30" y="52" font-size="26" fill="${D.chr.fill}" text-anchor="middle" font-family="serif" font-weight="bold">萬</text>`;
}

function windGlyph(dir: string, fg: string, size: number): string {
  const map: Record<string, string> = { E: "東", S: "南", W: "西", N: "北" };
  return `<text x="30" y="42" font-size="32" fill="${D.wind.fill}" text-anchor="middle" font-family="serif" font-weight="bold">${map[dir] ?? "?"}</text>`;
}

function flower(n: number, ac: string, size: number): string {
  const petals = [5, 6, 7, 8][n - 1] ?? 5;
  let out = "";
  for (let i = 0; i < petals; i++) {
    const a = (i / petals) * Math.PI * 2;
    out += `<ellipse cx="${30 + Math.cos(a) * 8}" cy="${30 + Math.sin(a) * 8}" rx="4.6" ry="7" fill="${ac}" opacity="0.9" transform="rotate(${(a * 180) / Math.PI} ${30 + Math.cos(a) * 8} ${30 + Math.sin(a) * 8})"/>`;
  }
  return `${out}<circle cx="30" cy="30" r="3.6" fill="#e8e0c8" stroke="#8a7a4a"/>`;
}

function season(n: number, ac: string, size: number): string {
  // stylized seasonal icons: bloom / sun / leaf / snow
  switch (n) {
    case 1:
      return flower(1, "#e88bb0", size);
    case 2:
      return `<circle cx="30" cy="30" r="11" fill="#e8b04b"/><g stroke="#e8b04b" stroke-width="2.4" stroke-linecap="round">${[0, 45, 90, 135, 180, 225, 270, 315]
        .map((a) => `<line x1="30" y1="13" x2="30" y2="8" transform="rotate(${a} 30 30)"/>`)
        .join("")}</g>`;
    case 3:
      return `<path d="M30 16 C42 22 42 38 30 46 C18 38 18 22 30 16 Z" fill="#6aa84f"/><line x1="30" y1="18" x2="30" y2="46" stroke="#3d6b2f" stroke-width="1.8"/>`;
    default:
      return `<g fill="#9fc3e8">${[
        [22, 24],
        [34, 22],
        [28, 32],
        [36, 36],
        [22, 40],
      ]
        .map(([x, y]) => `<circle cx="${x}" cy="${y}" r="4.4"/>`)
        .join("")}</g>`;
  }
}
