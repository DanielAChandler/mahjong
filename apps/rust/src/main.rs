//! Rust app entry: renders the board with pure-Rust logic (mahjong-core
//! linked directly — no wasm hop).

mod ui;

fn main() {
    ui::run();
}
