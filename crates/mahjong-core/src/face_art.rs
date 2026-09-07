//! Tile face art — THE single SVG generator, parameterized by theme id.
//! Consumed by the Rust app directly and by the TS app through wasm, so
//! both render pixel-identical faces. Face ids are the stable spec from
//! tiles.rs: dot1..9, bam1..9, chr1..9, windE/S/W/N, dragonR/G/W,
//! flower1..4, season1..4.

/// Palette values come from /shared/themes/themes.json (same ids).
pub fn face_svg(face_id: &str, theme_id: &str) -> String {
    let (fg, ac, flat) = theme_params(theme_id);
    let _ = fg;
    match () {
        _ if face_id.starts_with("dot") => dots(n(face_id, 3)),
        _ if face_id.starts_with("bam") => bams(n(face_id, 3)),
        _ if face_id.starts_with("chr") => chars(n(face_id, 3)),
        _ if face_id.starts_with("wind") => wind(face_id.chars().nth(4).unwrap_or('?')),
        _ if face_id.starts_with("dragon") => dragon(face_id),
        _ if face_id.starts_with("flower") => flower(n(face_id, 6), ac),
        _ if face_id.starts_with("season") => season(n(face_id, 6), ac),
        _ => format!(
            "<text x=\"30\" y=\"40\" font-size=\"24\" fill=\"{ac}\">{face_id}</text>"
        ),
    }
    .replace("FLAT", if flat { "none" } else { ac })
}

fn n(face_id: &str, from: usize) -> usize {
    face_id
        .get(from..)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

fn theme_params(theme_id: &str) -> (&'static str, &'static str, bool) {
    // (text/fg, accent, flat)
    match theme_id {
        "flat" => ("#2a3040", "#4f8cff", true),
        "midnight" => ("#e8e4ff", "#b18cff", false),
        _ => ("#4a3f2a", "#e8b04b", false),
    }
}

const DOT_FILL: &str = "#2f6db3";
const BAM_FILL: &str = "#2e8b57";
const CHR_FILL: &str = "#b3402e";
const WIND_FILL: &str = "#4a4a6a";

fn dots(k: usize) -> String {
    const P: &[(usize, &[(i32, i32)])] = &[
        (1, &[(30, 30)]),
        (2, &[(30, 16), (30, 44)]),
        (3, &[(16, 14), (30, 30), (44, 46)]),
        (4, &[(17, 17), (43, 17), (17, 43), (43, 43)]),
        (5, &[(17, 17), (43, 17), (30, 30), (17, 43), (43, 43)]),
        (
            6,
            &[(17, 14), (43, 14), (17, 30), (43, 30), (17, 46), (43, 46)],
        ),
        (
            7,
            &[(17, 12), (43, 12), (17, 27), (43, 27), (17, 42), (43, 42), (30, 51)],
        ),
        (
            8,
            &[(17, 12), (43, 12), (17, 26), (43, 26), (17, 40), (43, 40), (17, 52), (43, 52)],
        ),
        (
            9,
            &[
                (15, 12), (30, 12), (45, 12), (15, 30), (30, 30), (45, 30), (15, 48), (30, 48),
                (45, 48),
            ],
        ),
    ];
    let pts = P.iter().find(|(k_, _)| *k_ == k).map(|(_, p)| *p).unwrap_or(&[(30, 30)]);
    pts.iter()
        .map(|&(x, y)| {
            format!(
                "<circle cx=\"{x}\" cy=\"{y}\" r=\"6.5\" fill=\"{DOT_FILL}\" stroke=\"#ffffff\" stroke-width=\"1.4\"/><circle cx=\"{x}\" cy=\"{y}\" r=\"2.4\" fill=\"#ffffff\" opacity=\"0.85\"/>"
            )
        })
        .collect()
}

fn bams(k: usize) -> String {
    const P: &[(usize, &[(i32, i32)])] = &[
        (1, &[(30, 30)]),
        (2, &[(22, 18), (38, 42)]),
        (3, &[(20, 14), (30, 30), (40, 46)]),
        (4, &[(20, 16), (40, 16), (20, 44), (40, 44)]),
        (5, &[(20, 16), (40, 16), (30, 30), (20, 44), (40, 44)]),
        (
            6,
            &[(20, 14), (40, 14), (20, 30), (40, 30), (20, 46), (40, 46)],
        ),
        (
            7,
            &[(30, 12), (20, 27), (40, 27), (20, 42), (40, 42), (20, 54), (40, 54)],
        ),
        (
            8,
            &[(20, 12), (40, 12), (20, 25), (40, 25), (20, 38), (40, 38), (20, 51), (40, 51)],
        ),
        (
            9,
            &[
                (20, 12), (30, 12), (40, 12), (20, 30), (30, 30), (40, 30), (20, 48), (30, 48),
                (40, 48),
            ],
        ),
    ];
    let pts = P.iter().find(|(k_, _)| *k_ == k).map(|(_, p)| *p).unwrap_or(&[(30, 30)]);
    pts.iter()
        .map(|&(x, y)| {
            format!(
                "<g transform=\"translate({x},{y})\"><rect x=\"-2.2\" y=\"-7\" width=\"4.4\" height=\"14\" rx=\"2\" fill=\"{BAM_FILL}\"/><rect x=\"-2.2\" y=\"-7\" width=\"1.6\" height=\"14\" rx=\"0.8\" fill=\"#57b585\" opacity=\"0.7\"/></g>"
            )
        })
        .collect()
}

const CHRN: [&str; 9] = ["一", "二", "三", "四", "五", "六", "七", "八", "九"];

fn chars(k: usize) -> String {
    let num = CHRN.get(k - 1).copied().unwrap_or("?");
    format!(
        "<text x=\"30\" y=\"24\" font-size=\"19\" fill=\"{CHR_FILL}\" text-anchor=\"middle\" font-family=\"serif\" font-weight=\"bold\">{num}</text><text x=\"30\" y=\"52\" font-size=\"26\" fill=\"{CHR_FILL}\" text-anchor=\"middle\" font-family=\"serif\" font-weight=\"bold\">萬</text>"
    )
}

fn wind(dir: char) -> String {
    let g = match dir {
        'E' => "東",
        'S' => "南",
        'W' => "西",
        'N' => "北",
        _ => "?",
    };
    format!(
        "<text x=\"30\" y=\"42\" font-size=\"32\" fill=\"{WIND_FILL}\" text-anchor=\"middle\" font-family=\"serif\" font-weight=\"bold\">{g}</text>"
    )
}

fn dragon(face_id: &str) -> String {
    let (c, label) = if face_id.ends_with('R') {
        ("#b3402e", "中")
    } else if face_id.ends_with('G') {
        ("#2e8b57", "發")
    } else {
        ("#2f6db3", "白")
    };
    format!(
        "<text x=\"30\" y=\"42\" font-size=\"34\" fill=\"{c}\" text-anchor=\"middle\" font-family=\"serif\">{label}</text>"
    )
}

fn flower(k: usize, ac: &str) -> String {
    let petals = [5usize, 6, 7, 8].get(k - 1).copied().unwrap_or(5);
    let mut out = String::new();
    for i in 0..petals {
        let a = (i as f64 / petals as f64) * std::f64::consts::PI * 2.0;
        let cx = 30.0 + a.cos() * 8.0;
        let cy = 30.0 + a.sin() * 8.0;
        let deg = a * 180.0 / std::f64::consts::PI;
        out.push_str(&format!(
            "<ellipse cx=\"{cx:.2}\" cy=\"{cy:.2}\" rx=\"4.6\" ry=\"7\" fill=\"{ac}\" opacity=\"0.9\" transform=\"rotate({deg:.2} {cx:.2} {cy:.2})\"/>"
        ));
    }
    out.push_str("<circle cx=\"30\" cy=\"30\" r=\"3.6\" fill=\"#e8e0c8\" stroke=\"#8a7a4a\"/>");
    out
}

fn season(k: usize, _ac: &str) -> String {
    match k {
        1 => flower(1, "#e88bb0"),
        2 => {
            let rays: String = [0usize, 45, 90, 135, 180, 225, 270, 315]
                .iter()
                .map(|a| {
                    format!(
                        "<line x1=\"30\" y1=\"13\" x2=\"30\" y2=\"8\" transform=\"rotate({a} 30 30)\"/>"
                    )
                })
                .collect();
            format!(
                "<circle cx=\"30\" cy=\"30\" r=\"11\" fill=\"#e8b04b\"/><g stroke=\"#e8b04b\" stroke-width=\"2.4\" stroke-linecap=\"round\">{rays}</g>"
            )
        }
        3 => "<path d=\"M30 16 C42 22 42 38 30 46 C18 38 18 22 30 16 Z\" fill=\"#6aa84f\"/><line x1=\"30\" y1=\"18\" x2=\"30\" y2=\"46\" stroke=\"#3d6b2f\" stroke-width=\"1.8\"/>".into(),
        _ => {
            let snow: String = [(22, 24), (34, 22), (28, 32), (36, 36), (22, 40)]
                .iter()
                .map(|(x, y)| format!("<circle cx=\"{x}\" cy=\"{y}\" r=\"4.4\"/>"))
                .collect();
            format!("<g fill=\"#9fc3e8\">{snow}</g>")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_face_ids_produce_svg() {
        let ids: Vec<String> = (1..=9)
            .flat_map(|i| {
                [
                    format!("dot{i}"),
                    format!("bam{i}"),
                    format!("chr{i}"),
                ]
            })
            .chain(["windE", "windS", "windW", "windN", "dragonR", "dragonG", "dragonW"].into_iter().map(String::from))
            .chain((1..=4).flat_map(|i| [format!("flower{i}"), format!("season{i}")]))
            .collect();
        assert_eq!(ids.len(), 42);
        for id in ids {
            for theme in ["classic", "flat", "midnight"] {
                let svg = face_svg(&id, theme);
                assert!(svg.contains("<"), "{id}/{theme} produced empty svg");
                assert!(!svg.contains("FLAT"), "{id}/{theme} left placeholder");
            }
        }
    }
}
