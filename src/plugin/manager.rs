// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::vec::Vec;

use super::Plugin;
use super::lua_api::{LuaContext, initialize_lua_environment};
use crate::memory;
use crate::types::{EmuError, EmuResult, VERBOSE_ENABLED, Word};
use mlua::{Function, Lua};

/// Maps hook names to Lua functions
type ScriptHookMap = HashMap<String, String>;

/// Represents a loaded Lua script plugin
pub struct LuaPlugin {
    pub lua_vm: Lua,
    pub lua_ctx: LuaContext,
    pub _script_code: String,
    pub hook_map: ScriptHookMap,
}

pub struct PluginManager {
    pub plugins: Vec<Box<dyn Plugin>>,
    pub lua_plugins: Vec<LuaPlugin>,
    // lua_hook_queue: Vec<(usize, String)>,
    pub lua_hook_queue: RefCell<Vec<(usize, String)>>,
}

impl PluginManager {
    pub fn new(plugins: Vec<Box<dyn Plugin>>) -> Self {
        PluginManager {
            plugins,
            lua_plugins: Vec::new(),
            // lua_hook_queue: Vec::new(),
            lua_hook_queue: RefCell::new(Vec::new()),
        }
    }

    pub fn enqueue_lua_hooks(&self, hook_name: &str) {
        let mut q = self.lua_hook_queue.borrow_mut();
        for (i, _) in self.lua_plugins.iter().enumerate() {
            q.push((i, hook_name.to_string()));
        }
    }

    pub fn flush_lua_hooks(&self) -> EmuResult<()> {
        let mut q = self.lua_hook_queue.borrow_mut();
        let queued = std::mem::take(&mut *q); // take ownership, drop borrow before Lua runs
        drop(q);

        for (plugin_idx, hook_name) in queued {
            let plugin = &self.lua_plugins[plugin_idx];
            let globals = plugin.lua_vm.globals();

            if let Some(fn_name) = plugin.hook_map.get(&hook_name) {
                if let Ok(func) = globals.get::<Function>(fn_name.as_str()) {
                    if let Err(e) = func.call::<()>(()) {
                        return Err(EmuError::InternalError(format!(
                            "[PluginManager][{}] Error calling '{}' hook: {}",
                            plugin.lua_ctx.plugin_name, fn_name, e
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn load_lua_plugin(
        &mut self,
        file_path: &str,
        cpu_state: Rc<RefCell<crate::cpu::CpuState>>,
        plugin_name: String,
    ) -> EmuResult<()> {
        let script_code = fs::read_to_string(file_path).map_err(|e| {
            EmuError::IoError(format!("Failed to read Lua script {}: {}", file_path, e))
        })?;

        let lua = Lua::new();
        initialize_lua_environment(&lua)
            .map_err(|e| EmuError::InternalError(format!("Lua env init failed: {}", e)))?;

        let lua_ctx = LuaContext::new(cpu_state.clone(), plugin_name.clone());
        let globals = lua.globals();
        globals
            .set("cpu", lua.create_userdata(lua_ctx.clone())?)
            .map_err(|e| EmuError::InternalError(format!("Setting cpu userdata failed: {}", e)))?;

        lua.load(&script_code)
            .exec()
            .map_err(|e| EmuError::InternalError(format!("Lua initial execution failed: {}", e)))?;

        let hook_map = HashMap::from([
            ("on_plugin_load".to_string(), "on_plugin_load".to_string()),
            (
                "on_plugin_unload".to_string(),
                "on_plugin_unload".to_string(),
            ),
            (
                "pre_pc_increment".to_string(),
                "pre_pc_increment".to_string(),
            ),
            (
                "post_pc_increment".to_string(),
                "post_pc_increment".to_string(),
            ),
            ("pre_syscall".to_string(), "pre_syscall".to_string()),
            ("post_syscall".to_string(), "post_syscall".to_string()),
            ("pre_bl".to_string(), "pre_bl".to_string()),
            ("post_bl".to_string(), "post_bl".to_string()),
            ("pre_ret".to_string(), "pre_ret".to_string()),
            ("post_ret".to_string(), "post_ret".to_string()),
        ]);

        self.lua_plugins.push(LuaPlugin {
            lua_vm: lua,
            lua_ctx,
            _script_code: script_code,
            hook_map,
        });

        if VERBOSE_ENABLED.load(Ordering::Relaxed) {
            println!(
                "[PluginManager] Loaded Lua plugin '{}' from {}",
                plugin_name, file_path
            );
        }
        Ok(())
    }

    // Helper to enqueue + flush in one step
    pub fn run_hooks(&self, hook_name: &str) -> EmuResult<()> {
        self.enqueue_lua_hooks(hook_name);
        self.flush_lua_hooks().map_err(|e| {
            EmuError::InternalError(format!(
                "[PluginManager] Error running Lua hooks for '{}': {}",
                hook_name, e
            ))
        })
    }

    pub fn on_plugin_load(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
    ) -> EmuResult<()> {
        self.run_hooks("on_plugin_load")?;
        for plugin in &mut self.plugins {
            plugin.on_plugin_load(cpu_regs, &memory)?;
        }
        Ok(())
    }

    pub fn on_plugin_unload(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
    ) -> EmuResult<()> {
        self.run_hooks("on_plugin_unload")?;
        for plugin in &mut self.plugins {
            plugin.on_plugin_unload(cpu_regs, &memory)?;
        }
        Ok(())
    }

    pub fn pre_pc_increment(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
    ) -> EmuResult<bool> {
        self.run_hooks("pre_pc_increment")?;
        let mut should_skip = false;
        for plugin in &mut self.plugins {
            if plugin.pre_pc_increment(cpu_regs, &memory)? {
                should_skip = true;
            }
        }
        Ok(should_skip)
    }

    pub fn post_pc_increment(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
    ) -> EmuResult<()> {
        self.run_hooks("post_pc_increment")?;
        for plugin in &mut self.plugins {
            plugin.post_pc_increment(cpu_regs, &memory)?;
        }
        Ok(())
    }

    pub fn pre_syscall(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
        sys_call_num: Word,
    ) -> EmuResult<bool> {
        self.run_hooks("pre_syscall")?;
        let mut should_skip = false;
        for plugin in &mut self.plugins {
            if plugin.pre_syscall(cpu_regs, &memory, sys_call_num)? {
                should_skip = true;
            }
        }
        Ok(should_skip)
    }

    pub fn post_syscall(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
        sys_call_num: Word,
    ) -> EmuResult<()> {
        self.run_hooks("post_syscall")?;
        for plugin in &mut self.plugins {
            plugin.post_syscall(cpu_regs, &memory, sys_call_num)?;
        }
        Ok(())
    }

    pub fn pre_bl(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
        target_addr: Word,
    ) -> EmuResult<bool> {
        self.run_hooks("pre_bl")?;
        let mut should_skip = false;
        for plugin in &mut self.plugins {
            if plugin.pre_bl(cpu_regs, &memory, target_addr)? {
                should_skip = true;
            }
        }
        Ok(should_skip)
    }

    pub fn post_bl(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
        target_addr: Word,
    ) -> EmuResult<()> {
        self.run_hooks("post_bl")?;
        for plugin in &mut self.plugins {
            plugin.post_bl(cpu_regs, &memory, target_addr)?;
        }
        Ok(())
    }

    pub fn pre_ret(&mut self, cpu_regs: &[Word; 32], memory: memory::Memory) -> EmuResult<bool> {
        self.run_hooks("pre_ret")?;
        let mut should_skip = false;
        for plugin in &mut self.plugins {
            if plugin.pre_ret(cpu_regs, &memory)? {
                should_skip = true;
            }
        }
        Ok(should_skip)
    }

    pub fn post_ret(&mut self, cpu_regs: &[Word; 32], memory: memory::Memory) -> EmuResult<()> {
        self.run_hooks("post_ret")?;
        for plugin in &mut self.plugins {
            plugin.post_ret(cpu_regs, &memory)?;
        }
        Ok(())
    }
}
