//! Pairing helpers: force even per-group counts and take pairs from a pool.

use crate::rng::Pcg32;

/// Even out per-group counts so a face multiset pairs fully. Deterministic
/// when given a deterministic RNG. The flip step may swap some faces to
/// different faces of another group — accepted spec behavior.
pub fn make_pairable(pool: &[u8], rng: &mut Pcg32) -> Vec<u8> {
    let mut out = pool.to_vec();
    let mut counts = [0usize; 36];
    for &f in &out {
        counts[crate::tiles::match_group(f) as usize] += 1;
    }
    let mut odd_groups: Vec<usize> = (0..36).filter(|&g| counts[g] % 2 == 1).collect();
    rng.shuffle(&mut odd_groups);
    while odd_groups.len() >= 2 {
        let g1 = odd_groups.pop().unwrap();
        let g2 = odd_groups.pop().unwrap();
        // flip one tile of g1 into a face of g2
        let idx = out
            .iter()
            .position(|&f| crate::tiles::match_group(f) as usize == g1)
            .expect("group with odd count must have a tile");
        out[idx] = face_of_group(g2);
    }
    out
}

/// Any face belonging to `group`.
pub fn face_of_group(group: usize) -> u8 {
    match group {
        34 => 34, // flower
        35 => 38, // season
        g if g < 34 => g as u8,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{seed_from_id, Pcg32};

    #[test]
    fn makes_even_groups() {
        for seed in 1..20u64 {
            let mut rng = Pcg32::new(seed_from_id(seed), 3);
            let mut deck = crate::tiles::standard_deck();
            rng.shuffle(&mut deck);
            for n in [6usize, 36, 60, 144] {
                let pool: Vec<u8> = deck.iter().copied().take(n).collect();
                let p = make_pairable(&pool, &mut rng);
                assert_eq!(p.len(), n);
                let mut counts = [0usize; 36];
                for &f in &p {
                    counts[crate::tiles::match_group(f) as usize] += 1;
                }
                assert!(
                    counts.iter().all(|&c| c % 2 == 0),
                    "seed {seed} n {n}: odd group remains"
                );
            }
        }
    }
}
