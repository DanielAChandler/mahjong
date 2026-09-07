//! Board model: placement + freedom queries + undo-able removals.
//!
//! Freedom rule (canonical for both implementations; half-unit grid, tiles
//! are TILE_W×TILE_H = 2×2 half-units):
//! - blocked from above: a PRESENT higher-layer tile's footprint overlaps
//!   this tile's footprint at all (straddled = covered)
//! - horizontally blocked: PRESENT same-layer tiles touch BOTH the left and
//!   the right edge of this tile (no gap) with overlapping y-ranges
//! A move removes two free tiles whose faces match (flowers match any flower,
//! seasons any season).

use crate::layout::{CompiledLayout, SlotKey, TILE_H, TILE_W};

/// One removal record — enough to undo fully.
#[derive(Debug, Clone, Copy)]
pub struct Move {
    pub a: usize,
    pub b: usize,
    pub face_a: u8,
    pub face_b: u8,
}

#[derive(Clone)]
pub struct Board {
    pub layout: std::sync::Arc<CompiledLayout>,
    /// face id per slot index; u8::MAX once removed
    pub faces: Vec<u8>,
    pub remaining: usize,
    pub history: Vec<Move>,
}

impl Board {
    pub fn new(layout: std::sync::Arc<CompiledLayout>, faces: Vec<u8>) -> Self {
        debug_assert_eq!(faces.len(), layout.len());
        let remaining = faces.iter().filter(|&&f| f != u8::MAX).count();
        Board {
            layout,
            faces,
            remaining,
            history: Vec::new(),
        }
    }

    pub fn tile_at(&self, idx: usize) -> Option<u8> {
        match self.faces[idx] {
            u8::MAX => None,
            f => Some(f),
        }
    }

    fn cell_present(&self, x: i32, y: i32, z: u8) -> bool {
        match self.layout.slot_index(SlotKey::new(x, y, z)) {
            Some(i) => self.faces[i] != u8::MAX,
            None => false,
        }
    }

    /// Present same-layer tiles touch BOTH side edges (x-1 / x+TILE_W columns
    /// with y-overlap)? Vita rule: sandwiched = not free.
    fn layer_blocked(&self, idx: usize) -> bool {
        let key = self.layout.keys[idx];
        let z = key.z;
        let y0 = key.y;
        let y1 = key.y + TILE_H - 1;
        let mut left = false;
        let mut right = false;
        for (i, k) in self.layout.keys.iter().enumerate() {
            if k.z != z || i == idx || self.faces[i] == u8::MAX {
                continue;
            }
            // y-ranges overlap?
            if k.y + TILE_H <= y0 || y1 + 1 <= k.y {
                continue;
            }
            if k.x + TILE_W == key.x {
                left = true; // touches left edge
            }
            if key.x + TILE_W == k.x {
                right = true; // touches right edge
            }
        }
        left && right
    }

    /// Blocked from above: any present tile on a higher layer whose footprint
    /// overlaps ours (straddling covers).
    fn covered_above(&self, idx: usize) -> bool {
        let key = self.layout.keys[idx];
        for (i, k) in self.layout.keys.iter().enumerate() {
            if k.z <= key.z || i == idx {
                continue;
            }
            // footprint overlap?
            if k.x < key.x + TILE_W && key.x < k.x + TILE_W
                && k.y < key.y + TILE_H && key.y < k.y + TILE_H
                && self.faces[i] != u8::MAX
            {
                return true;
            }
        }
        false
    }

    /// Test/inspect helper: is any tile present directly above `idx`?
    pub fn covered_above_pub(&self, idx: usize) -> bool {
        self.covered_above(idx)
    }

    pub fn is_free(&self, idx: usize) -> bool {
        if self.faces[idx] == u8::MAX {
            return false;
        }
        !self.covered_above(idx) && !self.layer_blocked(idx)
    }

    /// All free slot indices.
    pub fn all_free(&self) -> Vec<usize> {
        (0..self.faces.len()).filter(|&i| self.is_free(i)).collect()
    }

    /// All currently-legal pairs: side-by-side free pairs plus vertical
    /// stacks (top free, bottom free modulo its cover). Delegates to
    /// `can_remove` so hints/deadlock and the move rule can never disagree.
    pub fn find_matches(&self) -> Vec<(usize, usize)> {
        let n = self.faces.len();
        let mut out = Vec::new();
        for a in 0..n {
            if self.faces[a] == u8::MAX {
                continue;
            }
            for b in (a + 1)..n {
                if self.can_remove(a, b) {
                    out.push((a, b));
                }
            }
        }
        out
    }

    /// Vertical relation: is `b` stacked over `a`? On the half-unit grid,
    /// "directly on top" = higher layer with footprint overlap (straddle).
    fn is_on_top(&self, a: usize, b: usize) -> bool {
        let ka = self.layout.keys[a];
        let kb = self.layout.keys[b];
        kb.z > ka.z
            && kb.x < ka.x + TILE_W
            && ka.x < kb.x + TILE_W
            && kb.y < ka.y + TILE_H
            && ka.y < kb.y + TILE_H
    }

    /// Free ignoring ONE partner tile (pairs vanish simultaneously, so the
    /// partner must not count as a blocker — neither its cover cell nor its
    /// side cells). Used for all pair-removal checks.
    fn free_modulo(&self, idx: usize, ignore: usize) -> bool {
        if self.faces[idx] == u8::MAX {
            return false;
        }
        let key = self.layout.keys[idx];
        // covered if any present higher-layer tile overlaps (ignoring partner)
        let covered = {
            let mut c = false;
            for (i, k) in self.layout.keys.iter().enumerate() {
                if k.z <= key.z || i == ignore {
                    continue;
                }
                if k.x < key.x + TILE_W
                    && key.x < k.x + TILE_W
                    && k.y < key.y + TILE_H
                    && key.y < k.y + TILE_H
                    && self.faces[i] != u8::MAX
                {
                    c = true;
                    break;
                }
            }
            c
        };
        if covered {
            return false;
        }
        !self.layer_blocked_modulo(key, ignore)
    }

    /// Horizontal blocking treating `ignore`'s cells as empty.
    fn layer_blocked_modulo(&self, key: SlotKey, ignore: usize) -> bool {
        let z = key.z;
        let y0 = key.y;
        let y1 = key.y + TILE_H - 1;
        let mut left = false;
        let mut right = false;
        for (i, k) in self.layout.keys.iter().enumerate() {
            if k.z != z || i == ignore || self.faces[i] == u8::MAX {
                continue;
            }
            if k.y + TILE_H <= y0 || y1 + 1 <= k.y {
                continue;
            }
            if k.x + TILE_W == key.x {
                left = true;
            }
            if key.x + TILE_W == k.x {
                right = true;
            }
        }
        left && right
    }

    /// Shared legality check for a candidate pair (used by find_matches
    /// and remove_pair so hints/deadlock never disagree with the move rule).
    pub fn can_remove(&self, a: usize, b: usize) -> bool {
        if self.faces[a] == u8::MAX
            || self.faces[b] == u8::MAX
            || !crate::tiles::faces_match(self.faces[a], self.faces[b])
        {
            return false;
        }
        if self.is_on_top(a, b) {
            self.is_free(b) && self.free_modulo(a, b)
        } else if self.is_on_top(b, a) {
            self.is_free(a) && self.free_modulo(b, a)
        } else {
            self.free_modulo(a, b) && self.free_modulo(b, a)
        }
    }

    /// Remove a free matching pair. Vertical stacks (one partner directly
    /// on the other) are legal: the top tile comes off first.
    pub fn remove_pair(&mut self, a: usize, b: usize) -> bool {
        if !self.can_remove(a, b) {
            return false;
        }
        let mv = Move {
            a,
            b,
            face_a: self.faces[a],
            face_b: self.faces[b],
        };
        self.faces[a] = u8::MAX;
        self.faces[b] = u8::MAX;
        self.remaining -= 2;
        self.history.push(mv);
        true
    }

    pub fn undo_pair(&mut self) -> bool {
        match self.history.pop() {
            Some(mv) => {
                self.faces[mv.a] = mv.face_a;
                self.faces[mv.b] = mv.face_b;
                self.remaining += 2;
                true
            }
            None => false,
        }
    }

    pub fn is_cleared(&self) -> bool {
        self.remaining == 0
    }

    pub fn is_stuck(&self) -> bool {
        !self.is_cleared() && self.find_matches().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{parse_layout_text, CompiledLayout};

    fn two_row_board(faces: Vec<u8>) -> Board {
        let text = "1111\n";
        let l = parse_layout_text("t", "T", text).unwrap();
        let mut lay = l;
        lay.slots.truncate(2);
        lay.tile_count = 2;
        let cl = std::sync::Arc::new(CompiledLayout::compile(lay).unwrap());
        assert_eq!(cl.len(), 2);
        Board::new(cl, faces)
    }

    #[test]
    fn two_adjacent_tiles_left_frees_right() {
        // [0][1]: initially 0 is free (left edge open), 1 blocked both sides?
        // 1's left neighbor 0 occupies cells; right edge open -> 1 free too.
        let mut b = two_row_board(vec![3, 3]);
        assert!(b.is_free(0));
        assert!(b.is_free(1));
        assert!(b.remove_pair(0, 1));
        assert!(b.is_cleared());
        b.undo_pair();
        assert_eq!(b.remaining, 2);
        assert!(b.is_free(0) && b.is_free(1));
    }

    #[test]
    fn mismatch_rejected() {
        let mut b = two_row_board(vec![0, 1]);
        assert!(!b.remove_pair(0, 1));
    }

    #[test]
    fn sandwich_block() {
        // eight tiles in a row: the middle six are blocked (occupied on both
        // sides), the outer two are free
        let text = "11111111\n";
        let l = parse_layout_text("t", "T", text).unwrap();
        let cl = std::sync::Arc::new(CompiledLayout::compile(l).unwrap());
        let mut b = Board::new(cl, vec![0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(b.is_free(0));
        for i in 1..7 {
            assert!(!b.is_free(i), "slot {i} should be blocked");
        }
        assert!(b.is_free(7));
        // removing the outer pair frees the neighbors
        assert!(b.remove_pair(0, 7));
        assert!(b.is_free(1));
    }
}
