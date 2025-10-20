pub mod lua_api;
pub mod manager;

// use crate::cpu::CpuState;
use crate::memory::Memory;
use crate::types::{EmuResult, Word};

// pub type PluginEntry = unsafe extern "C" fn() -> *mut dyn Plugin;

/// Trait defining the plugin interface. Each plugin can react to emulator events.
pub trait Plugin: Send + Sync {
    /// Returns the name of the plugin for logging/identification.
    fn _name(&self) -> &str;

    fn on_plugin_load(&mut self) -> EmuResult<()> {
        Ok(())
    }

    fn on_plugin_unload(&mut self) -> EmuResult<()> {
        Ok(())
    }

    /// Called before an instruction is fetched and executed.
    fn pre_execution_event(&mut self, _cpu_regs: &[Word; 31], _memory: &Memory) -> EmuResult<bool> {
        Ok(false)
    }

    /// Called after a successful instruction execution.
    fn post_execution_event(&mut self, _cpu_regs: &[Word; 31], _memory: &Memory) -> EmuResult<()> {
        Ok(())
    }

    /// Called before a system call is executed.
    fn pre_syscall_execution(
        &mut self,
        _cpu_regs: &[Word; 31],
        _memory: &Memory,
        _sys_call_num: Word,
    ) -> EmuResult<bool> {
        Ok(false)
    }

    /// Called after a system call has been executed.
    fn post_syscall_execution(
        &mut self,
        _cpu_regs: &[Word; 31],
        _memory: &Memory,
        _sys_call_num: Word,
    ) -> EmuResult<()> {
        Ok(())
    }
}

pub use manager::PluginManager;
