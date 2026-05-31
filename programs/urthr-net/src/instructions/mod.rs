// Each instruction module exposes a `handler` fn; we always call them by explicit
// module path (e.g. `initialize_protocol::handler`), so the glob re-exports below
// (which exist to bring the `#[derive(Accounts)]` structs into the program module's
// scope) collide harmlessly on the `handler` name. Silence that benign warning.
#![allow(ambiguous_glob_reexports)]

pub mod initialize_protocol;
pub mod register_publisher;
pub mod stake;
pub mod unstake;
pub mod create_campaign;
pub mod fund_campaign;
pub mod submit_claim;

pub use initialize_protocol::*;
pub use register_publisher::*;
pub use stake::*;
pub use unstake::*;
pub use create_campaign::*;
pub use fund_campaign::*;
pub use submit_claim::*;
