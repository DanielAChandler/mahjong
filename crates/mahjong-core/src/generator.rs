//! Reverse (backward) puzzle generation — the solvability guarantee.
//!
//! Start from an EMPTY board and repeatedly place valid pairs onto slots
//! free in the partial board. Placement order = REVERSE of a valid removal
//! order, so `solution` (placements reversed) is a verified full solution.
//!
//! Two-phase design (fast + always terminating):
//!  1. GEOMETRY: depth-first search pairing up slot indices so that every
//!     pair is free at its placement time. Faces never affect freedom, so
//!     this search depends only on the layout.
//!  2. FACES: assign one face-pair (a matching duo from the deck pool) to
//!     each slot-pair. This can never fail — the pool is made pairable
//!     beforehand and consumed two faces per slot-pair in order.
//!
//! All randomness flows through `Pcg32` seeded by `seed_from_id`; all
//! iteration orders derive from sorted structures, so puzzle #N is identical
//! everywhere, in both implementations.

use crate::board::Board;
use crate::layout::{CompiledLayout, SlotKey, TILE_W};
use crate::rng::{seed_from_id, Pcg32};
use crate::tiles::{distinct_groups_in_play, standard_deck};
use serde::{Deserialize, Serialize};

/// A generated puzzle: faces per compiled-layout slot index + solution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Puzzle {
    pub puzzle_id: u64,
    pub layout_id: String,
    /// face id per slot index (compiled layout order)
    pub faces: Vec<u8>,
    /// forward removal order: pairs of slot indices
    pub solution: Vec<(usize, usize)>,
    pub difficulty: Difficulty,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Difficulty {
    pub tile_count: u32,
    pub max_height: u8,
    pub distinct_groups: u32,
    /// 1..=100 composite
    pub rating: u32,
}

/// Deterministic layout rotation for infinite mode (catalog order).
pub fn layout_index_for_puzzle(n: u64, num_layouts: usize) -> usize {
    let mut x = n ^ 0x6A09_E667_F3BC_C90B;
    crate::rng::splitmix64(&mut x) as usize % num_layouts
}

/// Sample n faces from the deck (puzzle-RNG shuffle), then force even count
/// per match-group via `pairing::make_pairable` so the pool pairs cleanly.
fn sample_pool(rng: &mut Pcg32, n: usize) -> Vec<u8> {
    let mut deck = standard_deck();
    rng.shuffle(&mut deck);
    let pool: Vec<u8> = deck.into_iter().take(n).collect();
    crate::pairing::make_pairable(&pool, rng)
}

/// Face assignment for one slot-pair from the pool (deterministic scan):
/// removes the first matching duo and returns it as (face_for_a, face_for_b).
fn take_face_pair(pool: &mut Vec<u8>) -> Option<(u8, u8)> {
    for i in 0..pool.len() {
        for j in (i + 1)..pool.len() {
            if crate::tiles::faces_match(pool[i], pool[j]) {
                let a = pool.remove(j);
                let b = pool.remove(i);
                return Some((b, a));
            }
        }
    }
    None
}

/// Phase 1: derive a full removal order on the FILLED board with faces
/// ignored (any two free tiles pair). Reversing it gives placement pairs.
/// Greedy randomized peel with restarts — deterministic for a given seed.
fn solve_geometry(
    layout: &std::sync::Arc<CompiledLayout>,
    rng: &mut Pcg32,
    restarts: u32,
) -> Option<Vec<(usize, usize)>> {
    for _ in 0..restarts {
        if let Some(order) = peel_once(layout, rng) {
            return Some(order);
        }
    }
    None
}

fn peel_once(
    layout: &std::sync::Arc<CompiledLayout>,
    rng: &mut Pcg32,
) -> Option<Vec<(usize, usize)>> {
    let n = layout.len();
    let mut removed = vec![false; n];
    let mut order: Vec<(usize, usize)> = Vec::with_capacity(n / 2);
    for _ in 0..n / 2 {
        let free = free_indices(&removed, layout);
        if free.len() < 2 {
            return None; // dead end — caller restarts
        }
        let i = rng.range(free.len());
        let j = loop {
            let j = rng.range(free.len());
            if j != i {
                break j;
            }
        };
        removed[free[i]] = true;
        removed[free[j]] = true;
        order.push((free[i], free[j]));
    }
    Some(order)
}

/// Free on the filled-then-partially-removed board: nothing above + not
/// sandwiched (side cells with respect to the ORIGINAL layout neighbors).
fn free_indices(removed: &[bool], layout: &std::sync::Arc<CompiledLayout>) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 0..removed.len() {
        if removed[i] {
            continue;
        }
        let key = layout.keys[i];
        let up = crate::layout::upper_key(key);
        let covered = match layout.slot_index(up) {
            Some(u) => !removed[u],
            None => false,
        };
        if covered {
            continue;
        }
        let left = match layout.slot_index(SlotKey::new(key.x - TILE_W, key.y, key.z)) {
            Some(l) => !removed[l],
            None => false,
        };
        let right = match layout.slot_index(SlotKey::new(key.x + TILE_W, key.y, key.z)) {
            Some(r) => !removed[r],
            None => false,
        };
        if left && right {
            continue;
        }
        out.push(i);
    }
    out
}

/// Composite 1..=100 rating (stable formula — part of the spec).
pub fn difficulty_rating(tile_count: u32, max_height: u8, distinct_groups: u32) -> u32 {
    let t = (tile_count as f32 / 144.0) * 40.0;
    let h = (max_height.saturating_sub(1) as f32) * 15.0;
    let g = ((distinct_groups as f32 - 8.0) / 28.0).clamp(0.0, 1.0) * 30.0;
    ((t + h + g).round() as u32).clamp(1, 100)
}

/// Generate puzzle `id` on `layout`. Deterministic across platforms.
pub fn generate_puzzle_on(
    layout: std::sync::Arc<CompiledLayout>,
    puzzle_id: u64,
) -> Result<Puzzle, String> {
    let n = layout.len();
    if n % 2 != 0 {
        return Err(format!("layout {}: odd slot count", layout.layout.id));
    }
    let mut rng = Pcg32::new(seed_from_id(puzzle_id), 0x5DEECE66D);

    // Phase 1: geometry — peel with restarts (deterministic seed stream).
    // NOTE: the peel produces the FORWARD removal order directly.
    let solution = solve_geometry(&layout, &mut rng, 4096)
        .ok_or_else(|| format!("puzzle {puzzle_id}: could not fill layout"))?;

    // Phase 2: faces — one matching duo per slot-pair, pool order stable.
    let mut pool = sample_pool(&mut rng, n);
    let mut faces = vec![u8::MAX; n];
    for &(a, b) in &solution {
        let Some((fa, fb)) = take_face_pair(&mut pool) else {
            return Err("face pool exhausted".to_string());
        };
        faces[a] = fa;
        faces[b] = fb;
    }
    debug_assert!(pool.is_empty());

    let max_height = layout.keys.iter().map(|k| k.z).max().unwrap_or(1);
    let distinct = distinct_groups_in_play(&faces) as u32;
    let tile_count = n as u32;
    let rating = difficulty_rating(tile_count, max_height, distinct);
    let layout_id = layout.layout.id.clone();
    Ok(Puzzle {
        puzzle_id,
        layout_id,
        faces,
        solution,
        difficulty: Difficulty {
            tile_count,
            max_height,
            distinct_groups: distinct,
            rating,
        },
    })
}

/// Shuffle power-up: re-face the occupied slots into a NEW solvable
/// arrangement for the same multiset. Peel over the occupied geometry
/// (deadend-free guarantee comes from the same argument as generation);
/// deterministic given `salt`. None when <2 tiles remain.
pub fn shuffle_faces(board: &Board, salt: u64) -> Option<Vec<u8>> {
    let occupied: Vec<usize> = (0..board.faces.len())
        .filter(|&i| board.faces[i] != u8::MAX)
        .collect();
    if occupied.len() < 2 {
        return None;
    }
    let layout = board.layout.clone();
    let mut rng = Pcg32::new(
        crate::rng::splitmix64(&mut (salt | 1)),
        0x2545_F491_4F6C_DD1D,
    );
    // peel a fresh removal order over the FULL layout, then keep the order
    // restricted to the occupied subset (all occupied tiles come off in a
    // valid relative order — removed tiles are simply absent).
    let full = peel_once(&layout, &mut rng)?;
    let order: Vec<(usize, usize)> = full
        .into_iter()
        .filter(|(a, b)| board.faces[*a] != u8::MAX && board.faces[*b] != u8::MAX)
        .collect();
    if order.len() * 2 != occupied.len() {
        // peel paired an occupied tile with an already-removed slot; retry
        // with fresh salts a few times (deterministic sequence)
        for extra in 1..64u64 {
            let mut rng2 = Pcg32::new(
                crate::rng::splitmix64(&mut (salt.wrapping_mul(extra) | 1)),
                0x2545_F491_4F6C_DD1D,
            );
            if let Some(full2) = peel_once(&layout, &mut rng2) {
                let order2: Vec<(usize, usize)> = full2
                    .into_iter()
                    .filter(|(a, b)| {
                        board.faces[*a] != u8::MAX && board.faces[*b] != u8::MAX
                    })
                    .collect();
                if order2.len() * 2 == occupied.len() {
                    return assign_faces(&board, &order2, &mut rng2);
                }
            }
        }
        return None;
    }
    assign_faces(&board, &order, &mut rng)
}

/// Assign the board's own face multiset onto the slot-pairs (one duo per
/// pair, deterministic scan). Always succeeds: the multiset pairs by
/// construction of the game (every removal took a matching duo off).
fn assign_faces(
    board: &Board,
    order: &[(usize, usize)],
    rng: &mut Pcg32,
) -> Option<Vec<u8>> {
    let mut pool: Vec<u8> = (0..board.faces.len())
        .filter(|&i| board.faces[i] != u8::MAX)
        .map(|i| board.faces[i])
        .collect();
    rng.shuffle(&mut pool);
    let mut counts = [0usize; 36];
    for &f in &pool {
        counts[crate::tiles::match_group(f) as usize] += 1;
    }
    if counts.iter().any(|&c| c % 2 == 1) {
        return None; // cannot happen in legal play
    }
    let mut new_faces = vec![u8::MAX; board.faces.len()];
    for &(a, b) in order {
        let Some((fa, fb)) = take_face_pair(&mut pool) else {
            return None;
        };
        new_faces[a] = fa;
        new_faces[b] = fb;
    }
    Some(new_faces)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{parse_layout_text, CompiledLayout};

    fn mini() -> std::sync::Arc<CompiledLayout> {
        let text = "1111\n1111\n---\n.11.\n";
        let l = parse_layout_text("mini", "Mini", text).unwrap();
        std::sync::Arc::new(CompiledLayout::compile(l).unwrap())
    }

    #[test]
    fn generates_and_replays() {
        let lay = mini();
        let p = generate_puzzle_on(lay.clone(), 25).unwrap();
        assert_eq!(p.faces.len(), 10);
        assert_eq!(p.solution.len(), 5);
        let mut b = crate::board::Board::new(lay, p.faces.clone());
        for (a, z) in &p.solution {
            assert!(b.remove_pair(*a, *z), "solution pair ({a},{z}) illegal");
        }
        assert!(b.is_cleared());
    }

    #[test]
    fn deterministic() {
        let a = generate_puzzle_on(mini(), 25).unwrap();
        let b = generate_puzzle_on(mini(), 25).unwrap();
        assert_eq!(a.faces, b.faces);
        assert_eq!(a.solution, b.solution);
        let c = generate_puzzle_on(mini(), 26).unwrap();
        assert_ne!(a.faces, c.faces);
    }

    #[test]
    fn pool_always_pairs() {
        let mut rng = Pcg32::new(seed_from_id(7), 1);
        for n in [6usize, 36, 60, 144] {
            let mut pool = sample_pool(&mut rng, n);
            let mut ok = true;
            while !pool.is_empty() {
                if take_face_pair(&mut pool).is_none() {
                    ok = false;
                    break;
                }
            }
            assert!(ok, "pool of {n} failed to pair");
        }
    }
}
