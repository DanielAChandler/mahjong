# Mahjong Solitaire — dual implementation (Rust + TypeScript)

One game, two implementations, **one source of truth**. The puzzle
generator, solver, rules engine, layout definitions, campaign data, and
theme data are implemented **once** in Rust (`crates/mahjong-core`) and
consumed by both apps — the TypeScript app through a wasm binding, the Rust
app by linking the core directly. Puzzle #N produces the exact same board
(same tiles, same positions, same solution) in both.

## Repository layout

```
crates/
  mahjong-core/      # THE shared engine (single implementation)
    src/rng.rs       #   PCG32 + splitmix64 — deterministic, seeded
    src/layout.rs    #   layout DSL compiler + geometry + freedom rules
    src/tiles.rs     #   144-tile deck spec, match groups
    src/pairing.rs   #   even-per-group pool correction
    src/generator.rs #   backward (peel) generator + shuffle power-up
    src/solver.rs    #   independent DFS solver (tests/CI)
    src/hints.rs     #   hint selection
    src/board.rs     #   play state: legality, removals, undo
    src/campaign.rs  #   curated level access (embedded JSON)
    src/themes.rs    #   embedded theme catalog
    src/bin/gen_campaign.rs  # generates /shared data; --check = CI gate
  mahjong-wasm/      # wasm-bindgen facade over core (TS app uses this)
shared/
  layouts/*.txt      # board shapes (editable ASCII DSL, 1 char = 1 tile)
  layouts/layouts.json      # compiled catalog (generated, committed)
  puzzle-spec/campaign.json # 300 curated levels (generated, committed)
  themes/themes.json        # theme + skin tokens (hand-written, shared)
  schemas/                  # reserved for JSON schema exports
apps/
  ts/                # TypeScript app (Vite + DOM renderer + wasm core)
  rust/              # Rust app (trunk + web-sys renderer, core linked)
```

## The "defined once" guarantee

- **Logic**: generator/solver/rules exist only in `mahjong-core`. The TS app
  has zero game logic — it calls `puzzle_for`, `state`, `can_remove`,
  `shuffle` through `crates/mahjong-wasm`.
- **Data**: `/shared/layouts`, `/shared/puzzle-spec`, `/shared/themes` are
  consumed by both apps (JSON import in TS, `include_str!` in Rust).
- **Determinism**: all randomness is PCG32 seeded via `seed_from_id(id)`;
  no platform RNG anywhere. `gen-campaign -- --check` regenerates the
  committed shared data and fails if it differs — so puzzle numbering can
  never drift.

## Layout DSL

A `.txt` file per board, one layer block per `---` separator (first block =
layer 1). One character = one tile; `.`/space = empty. Upper tiles must sit
exactly over a lower tile. Example (`spider.txt`):

```
11....11
11111111
..1111..
..1111..
11111111
11....11
----
....
....
..1111..
..1111..
```

## Build & test

Prereqs: Rust (stable + `wasm32-unknown-unknown`), [wasm-pack](https://rustwasm.github.io/wasm-pack/),
[trunk](https://trunkrs.dev), Node 22.

```sh
# core tests (rules, generator, solver, determinism)
cargo test -p mahjong-core

# regenerate/verify committed shared data
cargo run --release -p mahjong-core --bin gen-campaign -- --check

# TS app
cd apps/ts
npm install
wasm-pack build ../../crates/mahjong-wasm --target web --out-dir wasm-core --release
npm run check        # typecheck
npm run dev          # local dev server
npm run build        # production build -> dist/

# Rust app
cd apps/rust
trunk build --release   # -> dist/
trunk serve             # local dev server
```

## Deploy (GitHub Pages)

`.github/workflows/deploy.yml` builds both apps and deploys on push to
`main`:

- TS app: `https://<user>.github.io/mahjong/`
- Rust app: `https://<user>.github.io/mahjong/rust/`

`.github/workflows/ci.yml` runs on every PR: core tests, shared-data
determinism check, wasm build, TS typecheck+build, Rust app build.

Enable Pages once: repo → Settings → Pages → Source: **GitHub Actions**.

## Game features

- Classic turtle-style matching with layered boards; free = nothing on top
  and at least one side open; flowers match any flower, seasons any season.
- 10 layouts (turtle, pyramid, dragon, fortress, spider, cat, tower,
  butterfly, waterfall, arena), rotated deterministically in infinite mode.
- 300-level curated campaign + infinite puzzles (any id) + daily puzzle.
- Power-ups: hint (best-pair heuristic), shuffle (guaranteed-solvable
  re-deal), undo, dynamite (single-tile removal); coin economy with
  rewards for wins/streaks/daily login.
- Stats persisted to localStorage with JSON export/import for device moves.
- 3 themes × 3 board skins, swappable live. Responsive + PWA-installable
  (TS app), keyboard shortcuts on desktop (H/S/U/D).

## Solution verification

Every generated puzzle carries its full solution (a verified removal
order): generation is backward construction, so solvability is by
construction, and the campaign pipeline replays every level's solution as a
hard assertion. The independent DFS solver cross-checks a sample each run.
