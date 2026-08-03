//! Charging subsystem feature module.
//!
//! - `state` -- pure state machine types and current calculation
//! - `init`  -- HUSB238A hardware initialization helper
//! - `task`  -- I/O helpers: sensor reads, BQ24725 writes, full update cycle

pub mod init;
pub mod state;
pub mod task;

// `ChargeManager` is the only state type held by the RTIC runtime.
pub use state::ChargeManager;
