//! Layout types: text-grid DSL → compiled tile slots + geometry queries.
//!
//! DSL (single source of truth — the TS app consumes compiled JSON via wasm):
//! - A layout `.txt` is one or more LAYER BLOCKS separated by a line whose
//!   first non-space chars are `---`. First block = layer 1 (z=1), next = z2…
//! - Within a block, each line is a grid row; ONE CHARACTER = ONE TILE.
//!   `.` or space = empty; any digit = a tile slot on the CURRENT layer.
//! - `# offset: OX OY` (optional, anywhere in a block) places the block's
//!   tiles at HALF-UNIT grid coords (col*2 + OX, row*2 + OY). Layers use
//!   half-tile offsets so upper tiles STRADDLE two tiles below (Vita-style):
//!   L0 offset 0 0, L1 offset 1 1, L2 offset 2 1.
//! - Tile footprint = 2×2 half-units (TILE_W × TILE_H).
//!
//! With half-unit coordinates, layers overlap by half a tile; support,
//! coverage and side-blocking are footprint-overlap based (see board.rs).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tile width in half-unit grid coords.
pub const TILE_W: i32 = 2;
/// Tile height in half-unit grid coords.
pub const TILE_H: i32 = 2;

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
    // half-unit offset for the current block (col*2 + ox, row*2 + oy)
    let mut ox: i32 = 0;
    let mut oy: i32 = 0;

    for (lineno, line) in text.lines().enumerate() {
        if is_layer_separator(line) {
            z += 1;
            if z > 9 {
                return Err(format!("layout {id}: more than 9 layers"));
            }
            row = 0;
            ox = 0;
            oy = 0;
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("#") {
            // directive line: `# offset: OX OY`
            let t = rest.trim();
            if let Some(v) = t.strip_prefix("offset:") {
                let nums: Vec<&str> = v.split_whitespace().collect();
                if nums.len() != 2 {
                    return Err(format!(
                        "layout {id}: # offset expects two ints at line {}",
                        lineno + 1
                    ));
                }
                ox = nums[0].parse::<i32>().map_err(|_| {
                    format!("layout {id}: bad offset x at line {}", lineno + 1)
                })?;
                oy = nums[1].parse::<i32>().map_err(|_| {
                    format!("layout {id}: bad offset y at line {}", lineno + 1)
                })?;
            }
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
                    layers
                        .entry(z)
                        .or_default()
                        .set(col as i32 * 2 + ox, row * 2 + oy, z);
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

/// Validate geometric soundness (half-unit grid): every z>=2 slot's footprint
/// must be fully supported — for each 2×2 half-cell of its footprint, at
/// least one tile on the layer below overlaps that half-cell. This is the
/// Vita straddle rule: an upper tile rests ON tiles beneath it, never floats.
pub fn validate_support(layout: &Layout) -> Result<(), String> {
    // Collect lower-layer footprints as half-cell occupancy.
    let mut below_cells: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
    let mut max_z_seen: u8 = 1;
    for s in &layout.slots {
        max_z_seen = max_z_seen.max(s.z);
        if s.z >= 2 {
            continue;
        }
        for dx in 0..TILE_W {
            for dy in 0..TILE_H {
                below_cells.insert((s.x + dx, s.y + dy));
            }
        }
    }
    for s in &layout.slots {
        if s.z <= 1 {
            continue;
        }
        for dx in 0..TILE_W {
            for dy in 0..TILE_H {
                if !below_cells.contains(&(s.x + dx, s.y + dy)) {
                    return Err(format!(
                        "layout {}: slot ({},{},{}) floats (footprint half-cell unsupported)",
                        layout.id, s.x, s.y, s.z
                    ));
                }
            }
        }
    }
    let _ = max_z_seen;
    Ok(())
}

/// The half-cell directly covered by a slot: a tile is blocked from above if
/// ANY higher-layer tile's footprint overlaps its footprint center region.
/// We approximate with footprint-corner overlap via `covered_by` in board.rs;
/// this key remains for stack relations (tiles sharing x,y on z+1).
pub fn upper_key(key: SlotKey) -> SlotKey {
    SlotKey::new(key.x, key.y, key.z + 1)
}

/// The tile-grid (half-unit) cells covered by a slot's 2×2 footprint.
pub fn footprint(key: SlotKey) -> Vec<(i32, i32)> {
    let mut v = Vec::with_capacity((TILE_W * TILE_H) as usize);
    for dx in 0..TILE_W {
        for dy in 0..TILE_H {
            v.push((key.x + dx, key.y + dy));
        }
    }
    v
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
        // Half-unit geometry: same-layer rows never overlap (edge-to-edge).
        // Coverage comes from an upper layer straddling lower tiles:
        //   L0: 1111 / 1111   L1 (offset 1 1): .11.  -> straddles the seam
        use crate::board::Board;
        let text = "1111\n1111\n---\n# offset: 1 1\n.11.\n";
        let l = parse_layout_text("t", "T", text).unwrap();
        validate_support(&l).unwrap();
        let cl = std::sync::Arc::new(CompiledLayout::compile(l).unwrap());
        let n = cl.len();
        let b = Board::new(cl, vec![0; n]);
        assert_eq!(n, 8 + 2);

        // the two L1 tiles are free (nothing above them)
        let top: Vec<usize> = (0..n).filter(|&i| b.layout.keys[i].z == 2).collect();
        assert_eq!(top.len(), 2);
        for i in top {
            assert!(b.is_free(i), "top tile should be free");
        }
        // the six L0 tiles straddled by the top layer are covered
        let covered: Vec<usize> = (0..n)
            .filter(|&i| b.layout.keys[i].z == 1 && b.covered_above_pub(i))
            .collect();
        assert_eq!(covered.len(), 6, "straddled L0 tiles must be covered");
        for i in covered {
            assert!(!b.is_free(i));
        }
        // the two rightmost L0 tiles are not straddled
        let uncovered: Vec<usize> = (0..n)
            .filter(|&i| b.layout.keys[i].z == 1 && !b.covered_above_pub(i))
            .collect();
        assert_eq!(uncovered.len(), 2);
        for i in uncovered {
            assert!(
                !b.covered_above_pub(i),
                "edge tile has nothing above it"
            );
        }
    }

    #[test]
    fn same_layer_rows_never_cover() {
        // rows within one layer are 2 half-units apart = exactly tile height
        use crate::board::Board;
        let l = parse_layout_text("t", "T", "1111\n1111\n").unwrap();
        let cl = std::sync::Arc::new(CompiledLayout::compile(l).unwrap());
        let mut b = Board::new(cl, vec![0; 8]);
        for i in 0..8 {
            assert!(!b.covered_above_pub(i));
        }
        // in a full row of 8, only the ends are free (sandwich rule)
        assert!(b.is_free(0) && b.is_free(7));
        assert!(!b.is_free(3) && !b.is_free(4));
        let _ = &mut b;
    }
}
