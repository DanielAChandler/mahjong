//! Professional tile faces: the 碧海风 (Bihai feng) Wikimedia Commons set
//! (CC BY-SA 4.0), embedded as optimized inner-SVG fragments and recolored
//! per theme. Single source consumed by both apps.

use std::sync::OnceLock;

/// face_id -> inner SVG (no outer <svg>, colors = source palette)
fn faces() -> &'static serde_json::Value {
    static FACES: OnceLock<serde_json::Value> = OnceLock::new();
    FACES.get_or_init(|| {
        let raw = include_str!("../../../shared/tileart/faces.json");
        serde_json::from_str(raw).expect("faces.json invalid")
    })
}

/// The 9 source colors used by the artwork, in canonical order.
pub const SRC_PALETTE: [&str; 9] = [
    "#00082d", "#870000", "#003a37", "#560042", "#133f00", "#a00283", "#3b67e2", "#e07f00",
    "#bc00a1",
];

/// Complete tile face as an inner-SVG fragment, recolored for `theme_id`.
/// Themes may override any subset of the 9 source colors.
pub fn face_svg(face_id: &str, theme_id: &str) -> String {
    let base = faces()
        .get("faces")
        .and_then(|f| f.get(face_id))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if base.is_empty() {
        return format!(
            "<text x=\"50\" y=\"110\" font-size=\"40\" fill=\"#870000\">{face_id}</text>"
        );
    }
    let map = crate::themes::color_map(theme_id);
    let mut svg = base.to_string();
    if let Some(m) = map.as_ref() {
        for (from, to) in m {
            svg = svg.replace(from, to);
        }
    }
    svg
}

/// Attribution required by the CC BY-SA license.
pub const ATTRIBUTION: &str = "Tile art: 'Mahjong tiles' by 碧海风 (Bihai feng), CC BY-SA 4.0, via Wikimedia Commons";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_faces_present_and_recolored() {
        let ids: Vec<String> = (1..=9)
            .flat_map(|i| [format!("dot{i}"), format!("bam{i}"), format!("chr{i}")])
            .chain(
                ["windE", "windS", "windW", "windN", "dragonR", "dragonG", "dragonW"]
                    .iter()
                    .map(|s| s.to_string()),
            )
            .chain((1..=4).flat_map(|i| [format!("flower{i}"), format!("season{i}")]))
            .collect();
        assert_eq!(ids.len(), 42);
        for id in &ids {
            let svg = face_svg(id, "classic");
            assert!(svg.len() > 200, "{id} too small");
            assert!(svg.contains("<path"), "{id} has no paths");
        }
        // midnight theme must actually recolor
        let classic = face_svg("dot5", "classic");
        let midnight = face_svg("dot5", "midnight");
        assert_ne!(classic, midnight);
        // attribution exported
        assert!(ATTRIBUTION.contains("CC BY-SA"));
    }
}
