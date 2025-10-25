// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

pub mod alu;
pub mod state;

pub use state::CpuState;
pub use state::InterpretedProgram;

use crate::memory::Memory;
use crate::types::{EmuError, EmuResult, Word};

#[cfg(test)]
mod tests;
