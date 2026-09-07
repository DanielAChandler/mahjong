//! Campaign: fixed curated level sequence, loaded from /shared/puzzle-spec.
//! The same JSON drives both apps. Levels reference either a layout or a
//! generated puzzle id; the spec records both so the sequence is stable.

use crate::generator::{generate_puzzle_on, Puzzle};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// One campaign entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignLevel {
    /// 1-based position in the campaign
    pub level: u32,
    /// stable slug, e.g. "turtle-1"
    pub slug: String,
    pub title: String,
    /// layout to play on
    pub layout_id: String,
    /// puzzle id to generate on that layout (deterministic)
    pub puzzle_id: u64,
    /// curated difficulty 1..=5 (hand-tuned progression)
    pub difficulty: u8,
    /// design/practice note for maintainers (not shown in game)
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSpec {
    /// spec format version
    pub version: u32,
    pub name: String,
    pub levels: Vec<CampaignLevel>,
}

pub fn embedded_campaign() -> &'static CampaignSpec {
    static C: OnceLock<CampaignSpec> = OnceLock::new();
    C.get_or_init(|| {
        let raw = include_str!("../../../shared/puzzle-spec/campaign.json");
        serde_json::from_str(raw).expect("embedded campaign.json invalid")
    })
}

/// Build the concrete puzzle for campaign level `n` (1-based).
pub fn campaign_puzzle(n: u32) -> Result<Puzzle, String> {
    let spec = embedded_campaign();
    let lvl = spec
        .levels
        .iter()
        .find(|l| l.level == n)
        .ok_or_else(|| format!("campaign has no level {n}"))?;
    let layout = crate::layout::find_layout(&lvl.layout_id)
        .ok_or_else(|| format!("unknown layout {}", lvl.layout_id))?;
    generate_puzzle_on(layout, lvl.puzzle_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_spec_loads() {
        let spec = embedded_campaign();
        assert!(!spec.levels.is_empty());
        assert_eq!(spec.levels[0].level, 1);
        // strictly increasing levels
        for (a, b) in spec.levels.iter().zip(spec.levels.iter().skip(1)) {
            assert!(b.level > a.level);
        }
    }

    #[test]
    fn all_levels_generate() {
        let spec = embedded_campaign();
        for lvl in &spec.levels {
            let p = campaign_puzzle(lvl.level).unwrap_or_else(|e| {
                panic!("level {}: {}", lvl.level, e)
            });
            assert_eq!(p.faces.len() % 2, 0);
        }
    }
}
