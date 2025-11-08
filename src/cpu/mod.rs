// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

pub mod alu;
pub mod control_flow;
pub mod data_processing;
pub mod data_transfer;
pub mod flags;
pub mod state;
pub use state::{CpuState, InterpretedProgram};

#[cfg(test)]
mod tests;
