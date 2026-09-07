//! Original tile faces: clean, bold vector symbols (circles, sticks, and CJK
//! text) in the traditional mahjong style. Single source consumed by both apps.

use std::sync::OnceLock;

/// face_id -> inner SVG (no outer <svg>, colors = source palette)
fn faces() -> &'static serde_json::Value {
    static FACES: OnceLock<serde_json::Value> = OnceLock::new();
    FACES.get_or_init(|| {
        let raw = include_str!("../../../shared/tileart/faces.json");
        serde_json::from_str(raw).expect("faces.json invalid")
    })
}

/// Complete tile face as an inner-SVG fragment. All faces use a fixed
/// Vita-style palette (bold solid suit colors), so no per-theme recolor.
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

/// Attribution for the in-app credit.
pub const ATTRIBUTION: &str = "Tile art: original vector graphics using traditional mahjong motifs.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_faces_present_and_render() {
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
            assert!(svg.len() > 40, "{id} too small");
            assert!(
                svg.contains("<circle")
                    || svg.contains("<rect")
                    || svg.contains("<text")
                    || svg.contains("<path"),
                "{id} has no drawn symbol"
            );
        }
    }
}
