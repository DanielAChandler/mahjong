// Central registry of every wasm export the app uses.
import init, {
  mahjong_version,
  catalog,
  puzzle_for,
  campaign_puzzle,
  state,
  can_remove,
  shuffle,
  slot_coords,
} from "../wasm-core/mahjong_wasm.js";

export { init, mahjong_version, catalog, puzzle_for, campaign_puzzle, state, can_remove, shuffle, slot_coords };
