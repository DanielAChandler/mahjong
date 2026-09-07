//! gen-campaign: one-shot generator for committed shared data.
//!
//! - Embeds every /shared/layouts/*.txt into /shared/layouts/layouts.json
//! - Generates the curated campaign (levels 1..=300, difficulty curve),
//!   validating each level with the solver before committing.
//! - `--check` mode: regenerate and diff — CI runs this to verify the
//!   committed JSON is exactly what the current pipeline produces.

use mahjong_core::layout::{parse_layout_text, validate_support, Layout, LayoutCatalog};
use mahjong_core::solver::{SolveOutcome, Solver};
use mahjong_core::{generate_puzzle_on, Board};
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .canonicalize()
        .expect("repo root")
}

/// Difficulty curve for 300 levels: layout rotation + rating bands.
/// Early levels: small/easy layouts; later: big, deep, many-groups.
fn plan_for_level(level: u32, layout_ids: &[String]) -> (String, u64) {
    // layout: cycle small→large as levels progress; deterministic rotation
    let phase = match level {
        1..=30 => 0usize,    // tutorial: small layouts
        31..=100 => 1,       // medium
        101..=200 => 2,      // large
        _ => 3,              // everything, full rotation
    };
    // deterministic pseudo-rotation within phase (no RNG reuse issues)
    let mut x = (level as u64) ^ 0x9E37_79B9_7F4A_7C15;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    let (start, count) = match phase {
        0 => (0usize, 2.min(layout_ids.len())),
        1 => (0, 4.min(layout_ids.len())),
        2 => (2.min(layout_ids.len() - 1), 4.min(layout_ids.len())),
        _ => (0, layout_ids.len()),
    };
    let pick = start + (x as usize) % (count - start).max(1);
    let layout_id = layout_ids[pick.min(layout_ids.len() - 1)].clone();
    // puzzle id space per level: stable and non-overlapping-ish
    let puzzle_id = 1000 + (level as u64) * 7;
    (layout_id, puzzle_id)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let check = args.iter().any(|a| a == "--check");
    let root = repo_root();
    let layouts_dir = root.join("shared/layouts");
    let spec_dir = root.join("shared/puzzle-spec");
    let schema_dir = root.join("shared/schemas");

    // ---- 1. compile layout txt files ----
    let mut entries: Vec<(String, String)> = fs::read_dir(&layouts_dir)
        .expect("layouts dir")
        .filter_map(|e| {
            let p = e.ok()?.path();
            if p.extension()?.to_str()? == "txt" {
                let id = p.file_stem()?.to_str()?.to_string();
                Some((id, p.to_string_lossy().to_string()))
            } else {
                None
            }
        })
        .collect();
    entries.sort();

    let mut layouts: Vec<Layout> = Vec::new();
    for (id, path) in &entries {
        let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        let mut l = parse_layout_text(id, id, &text)
            .unwrap_or_else(|e| panic!("layout {id}: {e}"));
        validate_support(&l).unwrap_or_else(|e| panic!("layout {id}: {e}"));
        l.difficulty = ((l.tile_count / 36) + 1).min(5) as u8;
        layouts.push(l);
    }
    assert!(layouts.len() >= 8, "need >= 8 layouts, have {}", layouts.len());
    let layout_ids: Vec<String> = layouts.iter().map(|l| l.id.clone()).collect();

    let catalog = LayoutCatalog { layouts };
    let catalog_json = serde_json::to_string_pretty(&catalog).expect("catalog json");

    // In-memory compiled map (do NOT consult the committed embedded file —
    // this run is what produces the next committed file).
    let mut cl_map: std::collections::HashMap<String, std::sync::Arc<
        mahjong_core::layout::CompiledLayout,
    >> = std::collections::HashMap::new();
    for l in &catalog.layouts {
        let cl = mahjong_core::layout::CompiledLayout::compile(l.clone())
            .unwrap_or_else(|e| panic!("layout {}: {e}", l.id));
        cl_map.insert(l.id.clone(), std::sync::Arc::new(cl));
    }

    // ---- 2. campaign levels ----
    let mut levels: Vec<mahjong_core::campaign::CampaignLevel> = Vec::new();
    let mut full_solves = 0usize;
    for level in 1..=300u32 {
        let (layout_id, base_pid) = plan_for_level(level, &layout_ids);
        // Runtime control only: try a few puzzle ids per level (the id that
        // succeeds is COMMITTED, so puzzle numbering is stable from now on).
        let mut chosen: Option<(u64, mahjong_core::Puzzle)> = None;
        for attempt in 0..8u64 {
            let pid = base_pid + attempt;
            let layout = cl_map[&layout_id].clone();
            match generate_puzzle_on(layout, pid) {
                Ok(p) => {
                    chosen = Some((pid, p));
                    break;
                }
                Err(e) => {
                    if attempt == 7 {
                        panic!("level {level}: {e}");
                    }
                    let _ = e;
                }
            }
        }
        let (puzzle_id, puzzle) = chosen.unwrap();
        // verify the embedded solution replays (cheap, always)
        let layout = cl_map[&layout_id].clone();
        let mut b2 = Board::new(layout, puzzle.faces.clone());
        for (a, b) in &puzzle.solution {
            assert!(b2.remove_pair(*a, *b), "level {level} bad solution pair");
        }
        assert!(b2.is_cleared());
        // independent full solver on a sample (redundancy check — the
        // replay above is the constructive proof). Unsolvable = bug;
        // BudgetExhausted = acceptable on big boards (exponential search).
        if level % 10 == 1 || level == 300 {
            let layout = cl_map[&layout_id].clone();
            let board = Board::new(layout, puzzle.faces.clone());
            let mut solver = Solver::new(2_000_000);
            let report = solver.solve(&board);
            assert_ne!(
                report.outcome,
                SolveOutcome::Unsolvable,
                "level {level} UN SOLVABLE (nodes {})",
                report.nodes
            );
            let _ = full_solves;
            full_solves += 1;
        }

        let rating = puzzle.difficulty.rating;
        levels.push(mahjong_core::campaign::CampaignLevel {
            level,
            slug: format!("level-{level:03}"),
            title: format!("Level {level}"),
            layout_id,
            puzzle_id,
            difficulty: ((rating / 20) + 1).clamp(1, 5) as u8,
            note: String::new(),
        });
        if level % 50 == 0 {
            eprintln!("  validated {level}/300");
        }
    }

    // group titles by difficulty band for a nicer feel
    for lvl in levels.iter_mut() {
        lvl.title = match lvl.difficulty {
            1 => format!("Garden {}", lvl.level),
            2 => format!("Courtyard {}", lvl.level),
            3 => format!("Palace {}", lvl.level),
            4 => format!("Fortress {}", lvl.level),
            _ => format!("Citadel {}", lvl.level),
        };
    }

    let spec = mahjong_core::campaign::CampaignSpec {
        version: 1,
        name: "Curated Campaign".into(),
        levels,
    };
    let spec_json = serde_json::to_string_pretty(&spec).expect("spec json");

    if check {
        let old_cat = fs::read_to_string(layouts_dir.join("layouts.json")).unwrap_or_default();
        let old_spec = fs::read_to_string(spec_dir.join("campaign.json")).unwrap_or_default();
        let ok = old_cat == catalog_json && old_spec == spec_json;
        if !ok {
            eprintln!("--check FAILED: committed shared data is stale");
            std::process::exit(1);
        }
        println!("shared data up to date");
        return;
    }

    fs::create_dir_all(&schema_dir).ok();
    fs::write(layouts_dir.join("layouts.json"), &catalog_json).expect("write layouts.json");
    fs::write(spec_dir.join("campaign.json"), &spec_json).expect("write campaign.json");
    println!(
        "wrote {} layouts, {} campaign levels",
        entries.len(),
        spec.levels.len()
    );
}
