//! wasm bindings: the TS app consumes ALL game logic through this interface
//! so there is exactly one implementation of generator/solver/rules/hints.
//!
//! Convention: stateless functions over (layout_id, faces[]) — faces[i] is
//! the face id, or 255 once removed. All structs cross as plain JSON.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use mahjong_core::layout::CompiledLayout;
use mahjong_core::{Board, Puzzle};

fn layout_by_id(id: &str) -> Option<Arc<CompiledLayout>> {
    mahjong_core::layout::find_layout(id)
}

fn board_from(layout_id: &str, faces: &[u8]) -> Result<Board, JsValue> {
    let layout = layout_by_id(layout_id)
        .ok_or_else(|| JsValue::from_str(&format!("unknown layout {layout_id}")))?;
    if faces.len() != layout.len() {
        return Err(JsValue::from_str("faces length != layout slot count"));
    }
    Ok(Board::new(layout, faces.to_vec()))
}

#[wasm_bindgen]
pub fn mahjong_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Full catalog for menus: layouts + campaign levels.
#[wasm_bindgen]
pub fn catalog() -> Result<JsValue, JsValue> {
    #[derive(Serialize)]
    struct CatalogDto {
        layouts: Vec<LayoutInfo>,
        campaign: Vec<CampaignInfo>,
    }
    #[derive(Serialize)]
    struct LayoutInfo {
        id: String,
        name: String,
        tile_count: u32,
        layers: u8,
        difficulty: u8,
    }
    #[derive(Serialize)]
    struct CampaignInfo {
        level: u32,
        slug: String,
        title: String,
        layout_id: String,
        difficulty: u8,
    }

    let layouts = mahjong_core::layout::embedded_layouts()
        .iter()
        .map(|l| {
            let keys = &mahjong_core::layout::embedded_compiled();
            let layers = keys
                .iter()
                .find(|c| c.layout.id == l.id)
                .map(|c| c.max_z())
                .unwrap_or(1);
            LayoutInfo {
                id: l.id.clone(),
                name: l.name.clone(),
                tile_count: l.tile_count,
                layers,
                difficulty: l.difficulty,
            }
        })
        .collect();

    let campaign = mahjong_core::campaign::embedded_campaign()
        .levels
        .iter()
        .map(|l| CampaignInfo {
            level: l.level,
            slug: l.slug.clone(),
            title: l.title.clone(),
            layout_id: l.layout_id.clone(),
            difficulty: l.difficulty,
        })
        .collect();

    serde_wasm_bindgen::to_value(&CatalogDto { layouts, campaign })
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Generate the puzzle for `id` on `layout_id` (infinite mode).
#[wasm_bindgen]
pub fn puzzle_for(id: f64, layout_id: &str) -> Result<JsValue, JsValue> {
    let layout = layout_by_id(layout_id)
        .ok_or_else(|| JsValue::from_str(&format!("unknown layout {layout_id}")))?;
    let p: Puzzle = mahjong_core::generate_puzzle_on(layout, id as u64)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&p).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Generate the puzzle for campaign level `level` (1-based).
#[wasm_bindgen]
pub fn campaign_puzzle(level: f64) -> Result<JsValue, JsValue> {
    let p: Puzzle = mahjong_core::campaign::campaign_puzzle(level as u32)
        .map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&p).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Tile face SVG for `face_id` rendered in `theme_id` — shared by both apps.
#[wasm_bindgen]
pub fn face_svg(face_id: &str, theme_id: &str) -> JsValue {
    JsValue::from_str(&mahjong_core::face_art::face_svg(face_id, theme_id))
}

/// Slot coordinates (compiled layout order) for the renderer.
#[wasm_bindgen]
pub fn slot_coords(layout_id: &str) -> Result<JsValue, JsValue> {
    #[derive(Serialize)]
    struct Slot {
        x: i32,
        y: i32,
        z: u8,
    }
    let layout = layout_by_id(layout_id)
        .ok_or_else(|| JsValue::from_str(&format!("unknown layout {layout_id}")))?;
    let slots: Vec<Slot> = layout
        .keys
        .iter()
        .map(|k| Slot { x: k.x, y: k.y, z: k.z })
        .collect();
    serde_wasm_bindgen::to_value(&slots).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[derive(Serialize)]
struct StateDto {
    free: Vec<u32>,
    moves: Vec<[u32; 2]>,
    remaining: u32,
    stuck: bool,
    won: bool,
    hint: Option<[u32; 2]>,
}

/// Derive everything the UI needs for the current board.
#[wasm_bindgen]
pub fn state(layout_id: &str, faces: &[u8]) -> Result<JsValue, JsValue> {
    let board = board_from(layout_id, faces)?;
    let hint = mahjong_core::hints::best_hint(&board);
    let dto = StateDto {
        free: board
            .all_free()
            .into_iter()
            .map(|i| i as u32)
            .collect(),
        moves: board
            .find_matches()
            .iter()
            .map(|&(a, b)| [a as u32, b as u32])
            .collect(),
        remaining: board.remaining as u32,
        stuck: board.is_stuck(),
        won: board.is_cleared(),
        hint: hint.map(|(a, b)| [a as u32, b as u32]),
    };
    serde_wasm_bindgen::to_value(&dto).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Is removing (a,b) legal right now? (single source of truth for clicks)
#[wasm_bindgen]
pub fn can_remove(layout_id: &str, faces: &[u8], a: f64, b: f64) -> Result<bool, JsValue> {
    let board = board_from(layout_id, faces)?;
    Ok(board.can_remove(a as usize, b as usize))
}

/// Shuffle: re-face the occupied slots into a NEW solvable arrangement.
/// `salt` must increase each call (e.g. move count + attempt).
#[wasm_bindgen]
pub fn shuffle(layout_id: &str, faces: &[u8], salt: f64) -> Result<JsValue, JsValue> {
    let board = board_from(layout_id, faces)?;
    let new_faces: Vec<u8> =
        mahjong_core::shuffle_faces(&board, salt as u64).ok_or_else(|| {
            JsValue::from_str("no solvable reshuffle found for this multiset")
        })?;
    serde_wasm_bindgen::to_value(&new_faces).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[derive(Deserialize, Serialize)]
struct DealCheck {
    valid: bool,
    pairs: u32,
}

/// Deal validation: every match-group must have an even tile count.
#[wasm_bindgen]
pub fn validate_deal(faces: &[u8]) -> JsValue {
    let mut counts = [0u32; 36];
    let mut pairs = 0u32;
    for &f in faces {
        if f != u8::MAX {
            counts[mahjong_core::tiles::match_group(f) as usize] += 1;
        }
    }
    for &c in counts.iter() {
        if c >= 2 {
            pairs += c / 2;
        }
    }
    let valid = counts.iter().all(|&c| c % 2 == 0);
    serde_wasm_bindgen::to_value(&DealCheck { valid, pairs }).unwrap()
}
