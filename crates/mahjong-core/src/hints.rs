//! Hint selection: pick a "good" match among legal moves.
//!
//! Strategy (deterministic, no RNG): prefer pairs that unblock the most
//! buried tiles (score by freed neighbors + upper reliefs), tie-break by
//! slot order — so hints are stable for a given board state.

use crate::board::Board;

/// Score a candidate pair: number of tiles newly freed after removal.
fn unblock_score(board: &Board, a: usize, b: usize) -> i32 {
    let mut score = 0;
    let before: Vec<bool> = (0..board.faces.len())
        .map(|i| board.faces[i] != u8::MAX && board.is_free(i))
        .collect();
    let mut sim = board.clone();
    if !sim.remove_pair(a, b) {
        return i32::MIN;
    }
    for i in 0..board.faces.len() {
        if board.faces[i] == u8::MAX {
            continue;
        }
        let now_free = sim.is_free(i);
        if now_free && !before[i] {
            score += 1;
        }
    }
    score
}

/// Returns the best pair as (a, b), or None if no legal move exists.
pub fn best_hint(board: &Board) -> Option<(usize, usize)> {
    let matches = board.find_matches();
    if matches.is_empty() {
        return None;
    }
    let mut best: Option<(i32, (usize, usize))> = None;
    for &(a, b) in &matches {
        let s = unblock_score(board, a, b);
        // prefer higher score; ties keep the earlier pair (slot-order stable)
        let better = match best {
            None => true,
            Some((bs, _)) => s > bs,
        };
        if better {
            best = Some((s, (a, b)));
        }
    }
    best.map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{parse_layout_text, CompiledLayout};

    #[test]
    fn finds_pair_or_none() {
        let text = "1111\n";
        let l = parse_layout_text("t", "T", text).unwrap();
        let cl = std::sync::Arc::new(CompiledLayout::compile(l).unwrap());
        // 4 slots: faces 5,7,7,5
        let b = Board::new(cl.clone(), vec![5, 7, 7, 5]);
        let hint = best_hint(&b);
        assert!(hint.is_some());
        let (a, b2) = hint.unwrap();
        assert!(crate::tiles::faces_match(b.faces[a], b.faces[b2]));

        // unsolvable: no legal pair at all
        let b2 = Board::new(cl, vec![0, 1, 2, 3]);
        assert!(best_hint(&b2).is_none());
    }
}
