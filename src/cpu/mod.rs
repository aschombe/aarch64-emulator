// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

pub mod alu;
pub mod state;

pub use state::CpuState;
pub use state::InterpretedProgram;

#[cfg(test)]
mod tests;
