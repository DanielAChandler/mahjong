//! Shared theme data — include_str! of /shared/themes/themes.json so the
//! Rust app renders with EXACTLY the same theme definitions as the TS app.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    #[serde(rename = "boardBg")]
    pub board_bg: String,
    #[serde(rename = "boardBg2")]
    pub board_bg2: String,
    #[serde(rename = "tileFace")]
    pub tile_face: String,
    #[serde(rename = "tileEdge")]
    pub tile_edge: String,
    #[serde(rename = "tileSide")]
    pub tile_side: String,
    pub accent: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub palette: Palette,
    #[serde(rename = "fontFamily")]
    pub font_family: String,
    pub style: String,
    /// Optional remap of the tile-art source palette (9 colors) for this theme.
    #[serde(rename = "colorMap", default, skip_serializing_if = "Option::is_none")]
    pub color_map: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skin {
    pub id: String,
    pub name: String,
    pub css: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeCatalog {
    pub version: u32,
    pub themes: Vec<Theme>,
    pub skins: Vec<Skin>,
}

pub fn embedded() -> &'static ThemeCatalog {
    use std::sync::OnceLock;
    static C: OnceLock<ThemeCatalog> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../shared/themes/themes.json"
        ))
        .expect("embedded themes.json invalid")
    })
}

/// Tile-art color remap for a theme (face colors), if it defines one.
pub fn color_map(theme_id: &str) -> Option<Vec<(String, String)>> {
    embedded()
        .themes
        .iter()
        .find(|t| t.id == theme_id)
        .and_then(|t| t.color_map.as_ref())
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}
