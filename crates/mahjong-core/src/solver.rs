//! Solver: verify solvability of an arbitrary deal (independent of the
//! generator), used by tests, CI, and the in-game hint/deadlock machinery.
//!
//! DFS over legal removals with memoization on board hashes and a node
//! budget. Reports solvable/unsolvable/exhausted.

use crate::board::Board;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveOutcome {
    /// Cleared the board; `solution` holds the removal order.
    Solved,
    /// Exhausted the search space: no solution exists.
    Unsolvable,
    /// Node budget exhausted before a verdict.
    BudgetExhausted,
}

pub struct SolveReport {
    pub outcome: SolveOutcome,
    pub nodes: u64,
    /// pairs of slot indices, in removal order (when Solved)
    pub solution: Vec<(usize, usize)>,
}

pub struct Solver {
    pub node_budget: u64,
    nodes: u64,
    seen: HashSet<u64>,
}

const SOLVER_DEFAULT_BUDGET: u64 = 400_000;

impl Default for Solver {
    fn default() -> Self {
        Solver {
            node_budget: SOLVER_DEFAULT_BUDGET,
            nodes: 0,
            seen: HashSet::new(),
        }
    }
}

fn board_hash(b: &Board) -> u64 {
    // FNV-1a over remaining faces (u8::MAX = removed)
    let mut h: u64 = 0xcbf29ce484222325;
    for &f in &b.faces {
        h ^= f as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

impl Solver {
    pub fn new(node_budget: u64) -> Self {
        Solver {
            node_budget,
            ..Default::default()
        }
    }

    pub fn solve(&mut self, board: &Board) -> SolveReport {
        self.nodes = 0;
        self.seen.clear();
        let mut board = board.clone();
        let mut solution = Vec::new();
        let outcome = self.dfs(&mut board, &mut solution);
        SolveReport {
            outcome,
            nodes: self.nodes,
            solution,
        }
    }

    fn dfs(&mut self, board: &mut Board, solution: &mut Vec<(usize, usize)>) -> SolveOutcome {
        self.nodes += 1;
        if self.nodes > self.node_budget {
            return SolveOutcome::BudgetExhausted;
        }
        if board.is_cleared() {
            return SolveOutcome::Solved;
        }
        let h = board_hash(board);
        if !self.seen.insert(h) {
            return SolveOutcome::Unsolvable; // visited an equivalent state
        }
        let matches = board.find_matches();
        if matches.is_empty() {
            return SolveOutcome::Unsolvable;
        }
        for (a, b) in matches {
            board.remove_pair(a, b);
            solution.push((a, b));
            match self.dfs(board, solution) {
                SolveOutcome::Solved => return SolveOutcome::Solved,
                SolveOutcome::BudgetExhausted => return SolveOutcome::BudgetExhausted,
                SolveOutcome::Unsolvable => {}
            }
            solution.pop();
            board.undo_pair();
        }
        SolveOutcome::Unsolvable
    }
}

/// Convenience: does this deal have at least one legal move?
pub fn has_moves(board: &Board) -> bool {
    !board.find_matches().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::generate_puzzle_on;
    use crate::layout::{parse_layout_text, CompiledLayout};

    fn mini() -> std::sync::Arc<CompiledLayout> {
        let text = "1111\n.11.\n";
        let l = parse_layout_text("mini", "Mini", text).unwrap();
        std::sync::Arc::new(CompiledLayout::compile(l).unwrap())
    }

    #[test]
    fn generated_puzzles_solve() {
        for id in [1u64, 25, 999] {
            let p = generate_puzzle_on(mini(), id).unwrap();
            let b = Board::new(mini(), p.faces);
            let mut s = Solver::default();
            let r = s.solve(&b);
            assert_eq!(r.outcome, SolveOutcome::Solved, "puzzle {id}");
        }
    }

    #[test]
    fn detects_unsolvable() {
        // 4 tiles in a row; the only matching pair (7,7) is sandwiched
        let text = "1111\n";
        let l = parse_layout_text("t", "T", text).unwrap();
        let cl = std::sync::Arc::new(CompiledLayout::compile(l).unwrap());
        let b = Board::new(cl, vec![5, 7, 7, 9]);
        let mut s = Solver::default();
        let r = s.solve(&b);
        assert_eq!(r.outcome, SolveOutcome::Unsolvable);
    }
}
