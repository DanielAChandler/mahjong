//! Rust app UI: DOM rendering via web-sys, game logic via mahjong-core.
//! Mirrors the TS app's rendering approach (positioned tile divs, CSS
//! transitions) and consumes the same compiled shared data.

use mahjong_core::{campaign_puzzle, generate_puzzle_on, Board};
use wasm_bindgen::prelude::*;
use web_sys::{window, Element, HtmlElement};

fn document() -> web_sys::Document {
    window().unwrap().document().unwrap()
}

const FACE_IDS: [&str; 42] = [
    "dot1", "dot2", "dot3", "dot4", "dot5", "dot6", "dot7", "dot8", "dot9", "bam1", "bam2", "bam3",
    "bam4", "bam5", "bam6", "bam7", "bam8", "bam9", "chr1", "chr2", "chr3", "chr4", "chr5", "chr6",
    "chr7", "chr8", "chr9", "windE", "windS", "windW", "windN", "dragonR", "dragonG", "dragonW",
    "flower1", "flower2", "flower3", "flower4", "season1", "season2", "season3", "season4",
];

pub struct AppState {
    pub board: Option<Board>,
    pub layout_id: String,
    pub selected: Option<usize>,
    pub puzzle_label: String,
    pub start_ms: f64,
}

thread_local! {
    static THEME_ID: std::cell::RefCell<&'static str> = const { std::cell::RefCell::new("classic") };
}

fn set_theme(id: &'static str) {
    THEME_ID.with(|t| *t.borrow_mut() = id);
}

thread_local! {
    static STATE: std::cell::RefCell<AppState> = std::cell::RefCell::new(AppState {
        board: None,
        layout_id: String::new(),
        selected: None,
        puzzle_label: String::new(),
        start_ms: 0.0,
    });
}

fn doc() -> web_sys::Document {
    document()
}

fn el(id: &str) -> Element {
    doc().get_element_by_id(id).expect(id)
}

pub fn run() {
    // panic hook for better wasm errors
    console_error_panic_hook::set_once();

    let body = doc().body().expect("no body");
    body.set_inner_html(
        r#"
<div id="app">
  <header id="topbar">
    <button id="btn-menu" class="icon-btn" title="Menu">&#9776;</button>
    <div id="hud"><span id="hud-level">Level 1</span><span id="hud-timer">0:00</span><span id="hud-pairs"></span></div>
    <div id="powerups">
      <button id="pu-hint" class="pu" title="Hint (H)"><span class="pu-icon">&#128161;</span></button>
      <button id="pu-shuffle" class="pu" title="Shuffle (S)"><span class="pu-icon">&#128256;</span></button>
      <button id="pu-undo" class="pu" title="Undo (U)"><span class="pu-icon">&#8617;</span></button>
    </div>
  </header>
  <main id="board"></main>
  <p id="credit" style="position:fixed;left:8px;bottom:calc(4px + env(safe-area-inset-bottom));font-size:9px;opacity:.55;color:#e8ecf4;z-index:5;pointer-events:none">Tile art: 碧海风, CC BY-SA 4.0 (Wikimedia Commons)</p>
  <div id="toast" hidden></div>
</div>
"#,
    );

    start_campaign(1);
    bind_toolbar();
    start_timer_loop();
}

fn start_campaign(level: u32) {
    match campaign_puzzle(level) {
        Ok(p) => {
            let layout = mahjong_core::layout::find_layout(&p.layout_id).expect("layout");
            let board = Board::new(layout, p.faces.clone());
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.board = Some(board);
                st.layout_id = p.layout_id.clone();
                st.selected = None;
                st.puzzle_label = format!("Level {level}");
                st.start_ms = now_ms();
            });
            el("hud-level").set_text_content(Some(&format!("Level {level}")));
            render_board();
        }
        Err(e) => web_sys::console::error_1(&e.into()),
    }
}

fn start_infinite(id: u64, layout_id: &str) {
    let layout = match mahjong_core::layout::find_layout(layout_id) {
        Some(l) => l,
        None => return,
    };
    match generate_puzzle_on(layout, id) {
        Ok(p) => {
            let layout = mahjong_core::layout::find_layout(&p.layout_id).expect("layout");
            let board = Board::new(layout, p.faces.clone());
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.board = Some(board);
                st.layout_id = p.layout_id.clone();
                st.selected = None;
                st.puzzle_label = format!("Puzzle #{id}");
                st.start_ms = now_ms();
            });
            el("hud-level").set_text_content(Some(&format!("Puzzle #{id}")));
            render_board();
        }
        Err(e) => web_sys::console::error_1(&e.into()),
    }
}

fn render_board() {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let board = st.board.as_mut().expect("board");
        let container = el("board");
        container.set_inner_html("");
        let inner = doc().create_element("div").unwrap();
        inner.set_class_name("board-inner");
        container.append_child(&inner).unwrap();

        let free: Vec<usize> = board.all_free();
        for i in 0..board.faces.len() {
            let face = board.faces[i];
            if face == u8::MAX {
                continue;
            }
            let key = board.layout.keys[i];
            let tile = doc().create_element("div").unwrap();
            tile.set_class_name("tile");
            if free.contains(&i) {
                tile.set_class_name("tile free");
            }
            tile.set_attribute("data-idx", &i.to_string()).unwrap();
            tile.set_attribute("data-face-id", FACE_IDS[face as usize]).unwrap();
            // position: x * TW + z*10, y * TH + (maxZ - z) * DZ
            let max_z = board.layout.max_z();
            let px = key.x * 56 + key.z as i32 * 10;
            let py = key.y * 72 + (max_z as i32 - key.z as i32) * 14;
            tile.set_attribute("style", &format!("left:{}px;top:{}px;", px, py))
                .unwrap();
            let svg = mahjong_core::face_art::face_svg(FACE_IDS[face as usize], THEME_ID.with(|t| *t.borrow()));
            let face_html = format!(
                r#"<div class="tile-side"></div><div class="tile-face" style="background:#f7f2e7"><svg viewBox="0 0 139.764 200" width="100%" height="100%">{}</svg></div>"#,
                svg
            );
            tile.set_inner_html(&face_html);
            {
                let idx = i;
                let closure = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
                    on_tile(idx);
                });
                (tile.dyn_ref::<HtmlElement>().unwrap())
                    .set_onclick(Some(closure.as_ref().unchecked_ref()));
                closure.forget();
            }
            inner.append_child(&tile).unwrap();
        }

        // fit board to viewport (mirrors TS renderer.fitToViewport)
        let max_x = board.layout.keys.iter().map(|k| k.x).max().unwrap_or(8);
        let max_y = board.layout.keys.iter().map(|k| k.y).max().unwrap_or(7);
        let max_z = board.layout.max_z();
        let bw = (max_x + 2) as f64 * 56.0 + 56.0;
        let bh = (max_y + 2) as f64 * 72.0 + 14.0 * (max_z as f64 + 1.0) + 56.0;
        inner.set_attribute(
            "style",
            &format!("width:{}px;height:{}px;transform-origin:center center;", bw, bh),
        )
        .unwrap();
        if let Some(host) = container.dyn_ref::<HtmlElement>() {
            let avail_w = host.client_width() as f64 - 8.0;
            let avail_h = host.client_height() as f64 - 8.0;
            if avail_w > 0.0 && avail_h > 0.0 {
                let scale = (avail_w / bw).min(avail_h / bh).min(1.6);
                inner
                    .set_attribute(
                        "style",
                        &format!(
                            "width:{}px;height:{}px;transform-origin:center center;transform:scale({});",
                            bw, bh, scale
                        ),
                    )
                    .unwrap();
            }
        }

        let pairs = board.remaining / 2;
        el("hud-pairs").set_text_content(Some(&format!("{pairs} pairs")));
    });
}

fn on_tile(idx: usize) {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let sel = st.selected;
        let board = st.board.as_mut().expect("board");
        if !board.is_free(idx) {
            return;
        }
        match sel {
            None => {
                st.selected = Some(idx);
                highlight_selection(Some(idx));
            }
            Some(sel) if sel == idx => {
                st.selected = None;
                highlight_selection(None);
            }
            Some(sel) => {
                if board.remove_pair(sel, idx) {
                    st.selected = None;
                    highlight_selection(None);
                    drop(st);
                    render_board();
                    STATE.with(|s| {
                        let st = s.borrow();
                        if st.board.as_ref().unwrap().is_cleared() {
                            toast("Cleared! 🎉");
                        } else if st.board.as_ref().unwrap().is_stuck() {
                            toast("No moves left — reload for a new puzzle");
                        }
                    });
                } else {
                    if board.is_free(idx) {
                        st.selected = Some(idx);
                        highlight_selection(Some(idx));
                    }
                }
            }
        }
    });
}

fn highlight_selection(idx: Option<usize>) {
    let inner = el("board");
    if let Ok(list) = inner.query_selector_all(".tile") {
        for i in 0..list.length() {
            let t = list.item(i).unwrap();
            let t_el = t.dyn_ref::<Element>().unwrap().clone();
            let di = t_el.get_attribute("data-idx").unwrap().parse::<usize>().unwrap();
            t_el.set_class_name(if Some(di) == idx {
                "tile selected"
            } else {
                "tile"
            });
        }
    }
}

fn bind_toolbar() {
    let hint = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
        STATE.with(|s| {
            let st = s.borrow();
            if let Some((a, b)) = mahjong_core::hints::best_hint(st.board.as_ref().unwrap()) {
                toast(&format!("Hint: tiles {} and {}", a, b));
            } else {
                toast("No moves available");
            }
        });
    });
    el("pu-hint")
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(hint.as_ref().unchecked_ref()));
    hint.forget();

    let undo = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            if st.board.as_mut().unwrap().undo_pair() {
                drop(st);
                render_board();
            }
        });
    });
    el("pu-undo")
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(undo.as_ref().unchecked_ref()));
    undo.forget();

    let menu = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
        // simple prompt-based infinite mode (parity scope: campaign + infinite)
        let w = window().unwrap();
        if let Ok(Some(v)) = w.prompt_with_message_and_default("Puzzle id (blank = campaign next)", "") {
            if v.is_empty() {
                start_campaign(1);
            } else if let Ok(n) = v.parse::<u64>() {
                start_infinite(n, "turtle");
            }
        }
    });
    el("btn-menu")
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(menu.as_ref().unchecked_ref()));
    menu.forget();
}

fn start_timer_loop() {
    let f = Closure::<dyn Fn()>::new(|| {
        STATE.with(|s| {
            let st = s.borrow();
            let secs = ((now_ms() - st.start_ms) / 1000.0) as u32;
            el("hud-timer").set_text_content(Some(&format!("{}:{:02}", secs / 60, secs % 60)));
        });
    });
    window()
        .unwrap()
        .set_interval_with_callback_and_timeout_and_arguments_0(f.as_ref().unchecked_ref(), 1000)
        .unwrap();
    f.forget();
}

fn toast(msg: &str) {
    let t = el("toast");
    t.set_text_content(Some(msg));
    t.set_attribute("hidden", "false").ok();
    let w = window().unwrap();
    let f = Closure::<dyn Fn()>::new(move || {
        el("toast").set_attribute("hidden", "true").ok();
    });
    w.set_timeout_with_callback_and_timeout_and_arguments_0(f.as_ref().unchecked_ref(), 2200)
        .ok();
    f.forget();
}

fn now_ms() -> f64 {
    window().unwrap().performance().unwrap().now()
}
