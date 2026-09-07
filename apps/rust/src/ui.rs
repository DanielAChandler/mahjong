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
    static SKIN_ID: std::cell::RefCell<&'static str> = const { std::cell::RefCell::new("felt") };
    static SOUND_ON: std::cell::RefCell<bool> = const { std::cell::RefCell::new(true) };
}

fn set_theme(id: &'static str) {
    THEME_ID.with(|t| *t.borrow_mut() = id);
    persist_settings();
}

fn set_skin(id: &'static str) {
    SKIN_ID.with(|t| *t.borrow_mut() = id);
    persist_settings();
}

fn set_sound(on: bool) {
    SOUND_ON.with(|t| *t.borrow_mut() = on);
    persist_settings();
}

fn sound_on() -> bool {
    SOUND_ON.with(|t| *t.borrow())
}

const LS_KEY: &str = "mahjong-rust.v1";

/// Persist theme/skin/sound to localStorage (matches the TS app's persistence).
fn persist_settings() {
    let theme = THEME_ID.with(|t| *t.borrow());
    let skin = SKIN_ID.with(|t| *t.borrow());
    let sound = SOUND_ON.with(|t| *t.borrow());
    let json = format!(
        r#"{{"theme":"{theme}","skin":"{skin}","sound":{sound}}}"#
    );
    if let Some(storage) = window().unwrap().local_storage().ok().flatten() {
        storage.set_item(LS_KEY, &json).ok();
    }
}

fn load_settings() {
    if let Some(storage) = window().unwrap().local_storage().ok().flatten() {
        if let Ok(Some(raw)) = storage.get_item(LS_KEY) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(t) = v.get("theme").and_then(|x| x.as_str()) {
                    let id: &'static str = Box::leak(t.to_string().into_boxed_str());
                    THEME_ID.with(|x| *x.borrow_mut() = id);
                }
                if let Some(s) = v.get("skin").and_then(|x| x.as_str()) {
                    let id: &'static str = Box::leak(s.to_string().into_boxed_str());
                    SKIN_ID.with(|x| *x.borrow_mut() = id);
                }
                if let Some(s) = v.get("sound").and_then(|x| x.as_bool()) {
                    SOUND_ON.with(|x| *x.borrow_mut() = s);
                }
            }
        }
    }
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
  <div id="menu-overlay" hidden>
    <div id="menu-panel">
      <h2>Menu</h2>
      <div class="menu-section">
        <h3>Layout</h3>
        <div id="menu-layouts"></div>
      </div>
      <div class="menu-section">
        <h3>Theme</h3>
        <div id="menu-themes"></div>
      </div>
      <div class="menu-section">
        <h3>Board skin</h3>
        <div id="menu-skins"></div>
      </div>
      <div class="menu-section">
        <h3>Sound</h3>
        <button id="menu-sound" class="menu-toggle">🔊 Sound: On</button>
      </div>
      <div class="menu-actions">
        <button id="menu-new">New game</button>
        <button id="menu-close">Close</button>
      </div>
      <p class="menu-credit">Tile art: original vector graphics (traditional mahjong motifs)</p>
    </div>
  </div>
  <div id="toast" hidden></div>
</div>
"#,
    );

    load_settings();
    apply_theme_shell();
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
        let max_z = board.layout.max_z();
        let half_pad = 12.0_f64; // 24/2 — content centered inside the padded box
        let theme = mahjong_core::themes::embedded()
            .themes
            .iter()
            .find(|t| t.id == THEME_ID.with(|t| *t.borrow()))
            .cloned()
            .unwrap_or_else(|| mahjong_core::themes::embedded().themes[0].clone());
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
            // position: edge-to-edge portrait grid (x*54, y*70) + small z lift
            let px = key.x as f64 * 54.0 + key.z as f64 * 6.0 + half_pad;
            let py = key.y as f64 * 70.0 + (max_z as f64 - key.z as f64) * 9.0 + half_pad;
            let zidx = key.z as i32 * 1000 + key.y as i32 * 32 + key.x as i32;
            tile.set_attribute(
                "style",
                &format!(
                    "left:{}px;top:{}px;z-index:{};--tile-edge:{};--tile-side:{};",
                    px, py, zidx, theme.palette.tile_edge, theme.palette.tile_side
                ),
            )
            .unwrap();
            let svg = mahjong_core::face_art::face_svg(FACE_IDS[face as usize], THEME_ID.with(|t| *t.borrow()));
            let face_html = format!(
                r#"<div class="tile-side"></div><div class="tile-face" style="background:{}"><svg viewBox="0 0 100 130" width="100%" height="100%">{}</svg></div>"#,
                theme.palette.tile_face, svg
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
        let z_spread = max_z as f64 * 6.0;
        // content box = content extent + uniform pad (centered via half_pad above)
        let bw = max_x as f64 * 54.0 + 54.0 + z_spread + 24.0;
        let bh = max_y as f64 * 70.0 + 70.0 + 9.0 * (max_z as f64 + 1.0) + 24.0;
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
                beep(520.0, 0.06, 0.10);
            }
            Some(sel) if sel == idx => {
                st.selected = None;
                highlight_selection(None);
            }
            Some(sel) => {
                if board.remove_pair(sel, idx) {
                    st.selected = None;
                    highlight_selection(None);
                    beep(660.0, 0.09, 0.14);
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
        open_menu();
    });
    el("btn-menu")
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(menu.as_ref().unchecked_ref()));
    menu.forget();

    let close = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
        el("menu-overlay").set_attribute("hidden", "").ok();
    });
    el("menu-close")
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(close.as_ref().unchecked_ref()));
    close.forget();

    let new_game = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
        el("menu-overlay").set_attribute("hidden", "").ok();
        start_campaign(1);
    });
    el("menu-new")
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(new_game.as_ref().unchecked_ref()));
    new_game.forget();

    // tap on the backdrop (outside the panel) closes the menu
    let backdrop = Closure::<dyn Fn(_)>::new(move |e: wasm_bindgen::JsValue| {
        let ev = e.unchecked_into::<web_sys::Event>();
        if ev.target() == ev.current_target() {
            el("menu-overlay").set_attribute("hidden", "").ok();
        }
    });
    el("menu-overlay")
        .dyn_ref::<HtmlElement>()
        .unwrap()
        .set_onclick(Some(backdrop.as_ref().unchecked_ref()));
    backdrop.forget();
}

/// Build + show the menu dialog: layouts from embedded catalog, themes from
/// embedded themes.json, both clickable.
fn open_menu() {
    let overlay = el("menu-overlay");

    // layouts
    let list = el("menu-layouts");
    list.set_inner_html("");
    for l in mahjong_core::layout::embedded_layouts().iter() {
        let id = l.id.clone();
        let btn = doc().create_element("button").unwrap();
        btn.set_text_content(Some(&l.name));
        btn.set_attribute("data-layout", &id).ok();
        let closure = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
            el("menu-overlay").set_attribute("hidden", "").ok();
            start_infinite(1, &id);
        });
        btn.dyn_ref::<HtmlElement>()
            .unwrap()
            .set_onclick(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
        list.append_child(&btn).unwrap();
    }

    // themes
    let tlist = el("menu-themes");
    tlist.set_inner_html("");
    let current_theme = THEME_ID.with(|t| *t.borrow());
    for t in mahjong_core::themes::embedded().themes.iter() {
        let id: &'static str = Box::leak(t.id.clone().into_boxed_str());
        let btn = doc().create_element("button").unwrap();
        btn.set_text_content(Some(&t.name));
        if t.id == current_theme {
            btn.set_class_name("active");
        }
        let closure = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
            set_theme(id);
            apply_theme_shell();
            render_board();
        });
        btn.dyn_ref::<HtmlElement>()
            .unwrap()
            .set_onclick(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
        tlist.append_child(&btn).unwrap();
    }

    // skins
    let slist = el("menu-skins");
    slist.set_inner_html("");
    let current_skin = SKIN_ID.with(|t| *t.borrow());
    for s in mahjong_core::themes::embedded().skins.iter() {
        let id: &'static str = Box::leak(s.id.clone().into_boxed_str());
        let btn = doc().create_element("button").unwrap();
        btn.set_text_content(Some(&s.name));
        if s.id == current_skin {
            btn.set_class_name("active");
        }
        let closure = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
            set_skin(id);
            apply_theme_shell();
        });
        btn.dyn_ref::<HtmlElement>()
            .unwrap()
            .set_onclick(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
        slist.append_child(&btn).unwrap();
    }

    // sound toggle (rebuild label each open)
    let sound_btn = el("menu-sound");
    sound_btn.set_text_content(Some(&format!(
        "🔊 Sound: {}",
        if sound_on() { "On" } else { "Off" }
    )));
    {
        let closure = Closure::<dyn Fn(_)>::new(move |_: wasm_bindgen::JsValue| {
            set_sound(!sound_on());
            let b = el("menu-sound");
            b.set_text_content(Some(&format!(
                "🔊 Sound: {}",
                if sound_on() { "On" } else { "Off" }
            )));
        });
        sound_btn
            .dyn_ref::<HtmlElement>()
            .unwrap()
            .set_onclick(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    overlay.remove_attribute("hidden").ok();
}

/// Apply current theme to the app shell (background, accent).
fn apply_theme_shell() {
    let theme = mahjong_core::themes::embedded()
        .themes
        .iter()
        .find(|t| t.id == THEME_ID.with(|t| *t.borrow()))
        .cloned()
        .unwrap_or_else(|| mahjong_core::themes::embedded().themes[0].clone());
    let skin_id = SKIN_ID.with(|t| *t.borrow());
    let skin_css = mahjong_core::themes::embedded()
        .skins
        .iter()
        .find(|s| s.id == skin_id && s.id != "felt")
        .map(|s| s.css.clone());

    if let Some(b) = doc().get_element_by_id("board") {
        let st = b.dyn_ref::<HtmlElement>().unwrap().style();
        if let Some(css) = skin_css {
            st.set_property("background", &css).ok();
        } else {
            // drive the CSS skin vars so the vignette overlay stays intact
            st.set_property("--skin-a", &theme.palette.board_bg).ok();
            st.set_property("--skin-b", &theme.palette.board_bg2).ok();
            st.set_property("background", "").ok();
        }
    }
    if let Some(h) = doc().get_element_by_id("topbar") {
        h.dyn_ref::<HtmlElement>()
            .unwrap()
            .style()
            .set_property("border-bottom", &format!("2px solid {}", theme.palette.accent))
            .ok();
    }
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
    t.remove_attribute("hidden").ok();
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

/// Minimal WebAudio beep (matches the TS app's synth feedback).
fn audio_ctx() -> Option<web_sys::AudioContext> {
    thread_local! {
        static CTX: std::cell::RefCell<Option<web_sys::AudioContext>> =
            const { std::cell::RefCell::new(None) };
    }
    CTX.with(|c| {
        if c.borrow().is_none() {
            *c.borrow_mut() = web_sys::AudioContext::new().ok();
        }
        c.borrow().clone()
    })
}

fn beep(freq: f32, dur: f32, gain: f32) {
    if !sound_on() {
        return;
    }
    let Some(ctx) = audio_ctx() else { return };
    let now = ctx.current_time();
    let (Ok(osc), Ok(g)) = (ctx.create_oscillator(), ctx.create_gain()) else {
        return;
    };
    osc.frequency().set_value(freq);
    g.gain().set_value(gain);
    g.gain()
        .exponential_ramp_to_value_at_time(0.001, now + dur as f64)
        .ok();
    osc.connect_with_audio_node(&g).ok();
    g.connect_with_audio_node(&ctx.destination()).ok();
    osc.start().ok();
    osc.stop_with_when(now + dur as f64).ok();
}
