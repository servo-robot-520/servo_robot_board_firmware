//! Vertically organized board features.
//!
//! Each feature owns its state, initialization, and task implementation.
//! Features communicate through protocol state data or by main.rs coordination.

pub mod charge;
pub mod communication;
pub mod power;
pub mod sensing;
pub mod servo;
pub mod telemetry;
