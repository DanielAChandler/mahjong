//! Tile faces and the standard 144-tile deck.

use serde::{Deserialize, Serialize};

/// Base suit faces (4 copies each in a deck): three suits + honors.
/// Display order/ids are part of the stable spec (theme art is keyed by id).
pub const BASE_FACES: &[&str] = &[
    // dots 1-9
    "dot1", "dot2", "dot3", "dot4", "dot5", "dot6", "dot7", "dot8", "dot9",
    // bamboo 1-9
    "bam1", "bam2", "bam3", "bam4", "bam5", "bam6", "bam7", "bam8", "bam9",
    // characters 1-9
    "chr1", "chr2", "chr3", "chr4", "chr5", "chr6", "chr7", "chr8", "chr9",
    // winds
    "windE", "windS", "windW", "windN",
    // dragons
    "dragonR", "dragonG", "dragonW",
];

/// Flower/season faces (1 copy each): any flower matches any other flower,
/// any season matches any other season.
pub const FLOWER_FACES: &[&str] = &["flower1", "flower2", "flower3", "flower4"];
pub const SEASON_FACES: &[&str] = &["season1", "season2", "season3", "season4"];

pub const FLOWER_GROUP: u8 = 34;
pub const SEASON_GROUP: u8 = 35;

/// Total faces with an identity: 34 base + 8 flower/season.
pub const FACE_KINDS: usize = 42;

/// Match group: two faces match iff same group id.
/// Base faces match only themselves; flowers form one group, seasons another.
pub fn match_group(face: u8) -> u8 {
    if (face as usize) < 34 {
        face
    } else if (face as usize) < 38 {
        FLOWER_GROUP
    } else {
        SEASON_GROUP
    }
}

pub fn faces_match(a: u8, b: u8) -> bool {
    match_group(a) == match_group(b)
}

/// The standard 144-tile deck: 4 of each base face, 1 of each flower/season.
pub fn standard_deck() -> Vec<u8> {
    let mut deck = Vec::with_capacity(144);
    for f in 0..34u8 {
        for _ in 0..4 {
            deck.push(f);
        }
    }
    for f in 34..42u8 {
        deck.push(f);
    }
    deck
}

/// How many distinct match-groups are in play for a face multiset.
pub fn distinct_groups_in_play(faces: &[u8]) -> usize {
    let mut seen = [false; 36];
    for &f in faces {
        seen[match_group(f) as usize] = true;
    }
    seen.iter().filter(|&&s| s).count()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaceInfo {
    pub id: &'static str,
    pub group: u8,
}

/// Metadata for themes/art pipelines: id + match group per face.
pub fn all_faces() -> Vec<FaceInfo> {
    let mut v = Vec::with_capacity(FACE_KINDS);
    for (i, id) in BASE_FACES.iter().enumerate() {
        v.push(FaceInfo { id, group: i as u8 });
    }
    for id in FLOWER_FACES {
        v.push(FaceInfo { id, group: FLOWER_GROUP });
    }
    for id in SEASON_FACES {
        v.push(FaceInfo { id, group: SEASON_GROUP });
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_size() {
        assert_eq!(standard_deck().len(), 144);
    }

    #[test]
    fn flowers_all_match() {
        assert!(faces_match(34, 37));
        assert!(!faces_match(34, 38));
        assert!(faces_match(38, 41));
        assert!(!faces_match(0, 0 + 1));
        assert!(faces_match(7, 7));
    }
}
