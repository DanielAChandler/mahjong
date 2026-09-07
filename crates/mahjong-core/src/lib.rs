//! mahjong-core: the single implementation of everything that must be
//! identical across the Rust and TypeScript apps: deterministic PRNG,
//! layout DSL compiler, backward generator, solver, hints, deck spec,
//! campaign spec. Consumed via wasm by the TS app, linked directly by
//! the Rust app.

pub mod board;
pub mod campaign;
pub mod generator;
pub mod hints;
pub mod layout;
pub mod pairing;
pub mod rng;
pub mod solver;
pub mod themes;
pub mod tiles;

pub use board::Board;
pub use campaign::{campaign_puzzle, CampaignLevel, CampaignSpec};
pub use generator::{generate_puzzle_on, shuffle_faces, Difficulty, Puzzle};
pub use layout::{embedded_compiled, embedded_layouts, CompiledLayout};
