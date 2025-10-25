pub mod lua_api;
pub mod manager;

// use crate::cpu::CpuState;
use crate::memory::Memory;
use crate::types::{EmuResult, Word};

#[cfg(test)]
mod tests;

// pub type PluginEntry = unsafe extern "C" fn() -> *mut dyn Plugin;

/// Trait defining the plugin interface. Each plugin can react to emulator events.
pub trait Plugin: Send + Sync {
    /// Returns the name of the plugin for logging/identification.
    fn _name(&self) -> &str;

    /// Called when the plugin is loaded.
    fn on_plugin_load(&mut self, _cpu_regs: &[Word; 32], _memory: &Memory) -> EmuResult<()> {
        Ok(())
    }

    /// Called when the plugin is unloaded.
    fn on_plugin_unload(&mut self, _cpu_regs: &[Word; 32], _memory: &Memory) -> EmuResult<()> {
        Ok(())
    }

    /// Called before an instruction is fetched and executed.
    fn pre_pc_increment(&mut self, _cpu_regs: &[Word; 32], _memory: &Memory) -> EmuResult<bool> {
        Ok(false)
    }

    /// Called after a successful instruction execution.
    fn post_pc_increment(&mut self, _cpu_regs: &[Word; 32], _memory: &Memory) -> EmuResult<()> {
        Ok(())
    }

    /// Called before a system call is executed.
    fn pre_syscall(
        &mut self,
        _cpu_regs: &[Word; 32],
        _memory: &Memory,
        _sys_call_num: Word,
    ) -> EmuResult<bool> {
        Ok(false)
    }

    /// Called after a system call has been executed.
    fn post_syscall(
        &mut self,
        _cpu_regs: &[Word; 32],
        _memory: &Memory,
        _sys_call_num: Word,
    ) -> EmuResult<()> {
        Ok(())
    }

    /// Called before a bl is executed.
    fn pre_bl(
        &mut self,
        _cpu_regs: &[Word; 32],
        _memory: &Memory,
        _target_addr: Word,
    ) -> EmuResult<bool> {
        Ok(false)
    }

    /// Called after a bl has been executed.
    fn post_bl(
        &mut self,
        _cpu_regs: &[Word; 32],
        _memory: &Memory,
        _target_addr: Word,
    ) -> EmuResult<()> {
        Ok(())
    }

    /// Called before a ret is executed.
    fn pre_ret(&mut self, _cpu_regs: &[Word; 32], _memory: &Memory) -> EmuResult<bool> {
        Ok(false)
    }

    /// Called after a ret has been executed.
    fn post_ret(&mut self, _cpu_regs: &[Word; 32], _memory: &Memory) -> EmuResult<()> {
        Ok(())
    }
}

pub use manager::PluginManager;
