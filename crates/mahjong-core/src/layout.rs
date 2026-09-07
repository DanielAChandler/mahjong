//! Layout types: text-grid DSL → compiled tile slots + geometry queries.
//!
//! DSL (single source of truth — the TS app consumes compiled JSON via wasm):
//! - A layout `.txt` is one or more LAYER BLOCKS separated by a line whose
//!   first non-space chars are `---`. First block = layer 1 (z=1), next = z2…
//! - Within a block, each line is a grid row; ONE CHARACTER = ONE TILE.
//!   `.` or space = empty; any digit = a tile slot on the CURRENT layer.
//! - Rows/cols are shared across blocks: align layers with leading dots —
//!   an upper tile must sit exactly over a lower tile (same column+row).
//! - Tile coords: x = col, y = row (1 char = 1 tile unit). The layout is
//!   normalized so min x/y are 0.
//!
//! With whole-tile coordinates, layers overlap exactly (half-tile offsets
//! are not expressible), so blocking/support use exact (x,y,z) matches.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tile width in grid units (DSL: one char per tile, so 1).
pub const TILE_W: i32 = 1;
/// Tile height in grid units.
pub const TILE_H: i32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileSlot {
    pub x: i32,
    pub y: i32,
    pub z: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub id: String,
    pub name: String,
    pub slots: Vec<TileSlot>,
    /// Authoring-time difficulty label 1..=5 (1 easiest).
    #[serde(default)]
    pub difficulty: u8,
    /// Number of tiles the layout holds (always even). Filled by the compiler.
    #[serde(default)]
    pub tile_count: u32,
}

/// Canonical slot coordinate key (tile grid coords).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SlotKey {
    pub x: i32,
    pub y: i32,
    pub z: u8,
}

impl SlotKey {
    pub fn new(x: i32, y: i32, z: u8) -> Self {
        SlotKey { x, y, z }
    }
}

/// Occupancy of one layer as a set of 1×1 grid cells.
#[derive(Debug, Clone, Default)]
pub struct LayerGrid {
    cells: BTreeMap<(i32, i32), u8>,
}

impl LayerGrid {
    pub fn set(&mut self, x: i32, y: i32, layer: u8) {
        self.cells.insert((x, y), layer);
    }
    pub fn get(&self, x: i32, y: i32) -> Option<u8> {
        self.cells.get(&(x, y)).copied()
    }
    pub fn is_free(&self, x: i32, y: i32) -> bool {
        !self.cells.contains_key(&(x, y))
    }
}

fn is_layer_separator(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("---")
}

/// Parse layout DSL text into a `Layout`. Errors describe the problem.
pub fn parse_layout_text(id: &str, name: &str, text: &str) -> Result<Layout, String> {
    let mut layers: BTreeMap<u8, LayerGrid> = BTreeMap::new();
    let mut z: u8 = 1;
    let mut row: i32 = 0;
    let mut any = false;

    for (lineno, line) in text.lines().enumerate() {
        if is_layer_separator(line) {
            z += 1;
            if z > 9 {
                return Err(format!("layout {id}: more than 9 layers"));
            }
            row = 0;
            continue;
        }
        let mut has_any_char = false;
        for (col, ch) in line.chars().enumerate() {
            has_any_char = true;
            match ch {
                '.' | ' ' => {}
                '-' => {
                    // bare dashes not forming a separator are an authoring error
                    if line.trim().chars().all(|c| c == '-') {
                        return Err(format!(
                            "layout {id}: near-separator dash line at line {} (use >=3 dashes)",
                            lineno + 1
                        ));
                    }
                    return Err(format!(
                        "layout {id}: stray '-' at line {} col {}",
                        lineno + 1,
                        col + 1
                    ));
                }
                c if c.is_ascii_digit() => {
                    any = true;
                    layers.entry(z).or_default().set(col as i32, row, z);
                }
                c => {
                    return Err(format!(
                        "layout {id}: bad char {c:?} at line {} col {}",
                        lineno + 1,
                        col + 1
                    ))
                }
            }
        }
        let _ = has_any_char;
        row += 1;
    }

    let mut slots: Vec<TileSlot> = Vec::new();
    for grid in layers.values() {
        for (&(cx, cy), &lz) in grid.cells.iter() {
            slots.push(TileSlot { x: cx, y: cy, z: lz });
        }
    }
    // normalize so min x/y are 0
    let min_x = slots.iter().map(|s| s.x).min().unwrap_or(0);
    let min_y = slots.iter().map(|s| s.y).min().unwrap_or(0);
    for s in slots.iter_mut() {
        s.x -= min_x;
        s.y -= min_y;
    }
    slots.sort_by_key(|s| (s.z, s.y, s.x));

    if !any || slots.is_empty() {
        return Err(format!("layout {id}: no slots"));
    }
    if slots.len() % 2 != 0 {
        return Err(format!("layout {id}: odd slot count {}", slots.len()));
    }

    Ok(Layout {
        id: id.to_string(),
        name: name.to_string(),
        tile_count: slots.len() as u32,
        slots,
        difficulty: 0,
    })
}

/// Validate geometric soundness: every z>=2 slot must sit exactly over a
/// slot on the layer below (same x,y — the DSL only expresses full offsets).
pub fn validate_support(layout: &Layout) -> Result<(), String> {
    use std::collections::HashSet;
    let mut by_layer: BTreeMap<u8, HashSet<(i32, i32)>> = BTreeMap::new();
    for s in &layout.slots {
        by_layer.entry(s.z).or_default().insert((s.x, s.y));
    }
    for s in &layout.slots {
        if s.z <= 1 {
            continue;
        }
        let below = by_layer.get(&(s.z - 1)).ok_or_else(|| {
            format!("layout {}: slot z{} has no layer below", layout.id, s.z)
        })?;
        if !below.contains(&(s.x, s.y)) {
            return Err(format!(
                "layout {}: slot ({},{},{}) floats (no support below)",
                layout.id, s.x, s.y, s.z
            ));
        }
    }
    Ok(())
}

/// Slot directly above: with the text-DSL's even grid coords, layers can
/// only overlap exactly (tiles are 2×2; half-tile offsets are not
/// expressible), so blocking/support is exact-footprint.
pub fn upper_key(key: SlotKey) -> SlotKey {
    SlotKey::new(key.x, key.y, key.z + 1)
}

/// The grid cell covered by a slot's footprint (1 char = 1 tile = 1 cell).
pub fn footprint(key: SlotKey) -> Vec<(i32, i32)> {
    vec![(key.x, key.y)]
}

/// Compiled layout: sorted slot list + lookup index + occupancy grids.
pub struct CompiledLayout {
    pub layout: Layout,
    /// Slots sorted by (z, y, x). Index of a slot == position here.
    pub keys: Vec<SlotKey>,
    index: BTreeMap<SlotKey, usize>,
    layers: BTreeMap<u8, LayerGrid>,
}

impl CompiledLayout {
    pub fn compile(layout: Layout) -> Result<Self, String> {
        let mut keys: Vec<SlotKey> = layout
            .slots
            .iter()
            .map(|s| SlotKey::new(s.x, s.y, s.z))
            .collect();
        keys.sort();
        keys.dedup();
        if keys.len() != layout.slots.len() {
            return Err(format!("layout {}: duplicate slots", layout.id));
        }
        if keys.len() % 2 != 0 {
            return Err(format!("layout {}: odd slot count", layout.id));
        }
        let mut index = BTreeMap::new();
        let mut layers: BTreeMap<u8, LayerGrid> = BTreeMap::new();
        for (i, k) in keys.iter().enumerate() {
            index.insert(*k, i);
            for (cx, cy) in footprint(*k) {
                layers.entry(k.z).or_default().set(cx, cy, k.z);
            }
        }
        Ok(CompiledLayout {
            layout,
            keys,
            index,
            layers,
        })
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn slot_index(&self, key: SlotKey) -> Option<usize> {
        self.index.get(&key).copied()
    }

    /// Is the 1×1 cell free at tile-grid coords on this layer? (Static
    /// layout occupancy — for presence checks use Board instead.)
    pub fn cell_free(&self, x: i32, y: i32, z: u8) -> bool {
        self.layers.get(&z).map_or(true, |l| l.is_free(x, y))
    }

    pub fn max_z(&self) -> u8 {
        self.keys.iter().map(|k| k.z).max().unwrap_or(0)
    }
}

/// Embedded compiled layout catalog, built from /shared/layouts/layouts.json
/// (generated by `gen-campaign`; committed so builds are hermetic).
pub fn embedded_layouts() -> &'static Vec<Layout> {
    static EMBEDDED: std::sync::OnceLock<Vec<Layout>> = std::sync::OnceLock::new();
    EMBEDDED.get_or_init(|| {
        let raw = include_str!("../../../shared/layouts/layouts.json");
        let cat: LayoutCatalog = serde_json::from_str(raw).expect("embedded layouts.json invalid");
        cat.layouts
    })
}

pub fn embedded_compiled() -> &'static Vec<std::sync::Arc<CompiledLayout>> {
    static EMBEDDED: std::sync::OnceLock<Vec<std::sync::Arc<CompiledLayout>>> =
        std::sync::OnceLock::new();
    EMBEDDED.get_or_init(|| {
        embedded_layouts()
            .iter()
            .map(|l| {
                std::sync::Arc::new(
                    CompiledLayout::compile(l.clone()).expect("embedded layout failed compile"),
                )
            })
            .collect()
    })
}

pub fn find_layout(id: &str) -> Option<std::sync::Arc<CompiledLayout>> {
    embedded_compiled()
        .iter()
        .find(|c| c.layout.id == id)
        .cloned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutCatalog {
    pub layouts: Vec<Layout>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRAPEZOID: &str = "\
....11....
...1111...
..111111..
.11111111.
";

    #[test]
    fn parses_and_normalizes() {
        let l = parse_layout_text("t", "T", TRAPEZOID).unwrap();
        assert_eq!(l.slots.len(), 2 + 4 + 6 + 8);
        assert_eq!(l.tile_count, l.slots.len() as u32);
        let c = CompiledLayout::compile(l).unwrap();
        assert_eq!(c.max_z(), 1);
    }

    #[test]
    fn multilayer_with_separator() {
        let text = "1111\n1111\n---\n.11.\n";
        let l = parse_layout_text("t", "T", text).unwrap();
        assert_eq!(l.slots.len(), 10);
        assert_eq!(l.slots.iter().filter(|s| s.z == 2).count(), 2);
        validate_support(&l).unwrap();
    }

    #[test]
    fn floating_slot_rejected() {
        // (0,0,z2) and (3,0,z2) have no tile directly below
        let text = ".11.\n1111\n---\n2..2\n";
        let l = parse_layout_text("t", "T", text).unwrap();
        assert!(validate_support(&l).is_err());
    }

    #[test]
    fn odd_count_rejected() {
        let e = parse_layout_text("t", "T", "11.\n.1.").unwrap_err();
        assert!(e.contains("odd"));
    }

    #[test]
    fn free_and_blocked() {
        // Board-level freedom on the trapezoid: top-row tiles are free;
        // a tile directly under a top-row tile is covered (not free).
        use crate::board::Board;
        let l = parse_layout_text("t", "T", TRAPEZOID).unwrap();
        let cl = std::sync::Arc::new(CompiledLayout::compile(l).unwrap());
        let n = cl.len();
        let b = Board::new(cl, vec![0; n]);
        // top row y=0: free
        let top: Vec<usize> = (0..n).filter(|&i| b.layout.keys[i].y == 0).collect();
        assert!(!top.is_empty());
        for i in top {
            assert!(b.is_free(i), "top tile {:?} should be free", b.layout.keys[i]);
        }
        // tiles in row 1 that sit directly under a top-row tile are covered
        let top_xs: std::collections::HashSet<i32> =
            (0..n).filter(|&i| b.layout.keys[i].y == 0).map(|i| b.layout.keys[i].x).collect();
        let under_top: Vec<usize> = (0..n)
            .filter(|&i| {
                let k = b.layout.keys[i];
                k.y == 1 && top_xs.contains(&k.x)
            })
            .collect();
        assert!(!under_top.is_empty());
        for i in under_top {
            assert!(
                !b.is_free(i),
                "covered tile {:?} should not be free",
                b.layout.keys[i]
            );
        }
        // and a row-1 tile sticking out beyond the top row is NOT covered
        // (it may still be side-blocked, so only sanity-check the flip side:
        // it must not be blocked from above)
        let outcrops: Vec<usize> = (0..n)
            .filter(|&i| {
                let k = b.layout.keys[i];
                k.y == 1 && !top_xs.contains(&k.x)
            })
            .collect();
        assert!(!outcrops.is_empty());
        for i in outcrops {
            let k = b.layout.keys[i];
            assert!(
                !b.covered_above_pub(i),
                "uncovered tile {k:?} has no tile above"
            );
        }
    }
}
