// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::Ordering;

use super::Plugin;
use super::lua_api::{create_snapshot, initialize_lua_environment};
use crate::types::{EmuError, EmuResult, VERBOSE_ENABLED};
use mlua::Lua;

type ScriptHookMap = HashMap<String, String>;

pub struct LuaPlugin {
    pub lua_vm: Lua,
    pub plugin_name: String,
    pub _script_code: String,
    pub hook_map: ScriptHookMap,
}

pub struct PluginManager {
    pub plugins: Vec<Box<dyn Plugin>>,
    pub lua_plugins: Vec<LuaPlugin>,
    pub lua_hook_queue: RefCell<Vec<(usize, String)>>,
}

impl PluginManager {
    pub fn new(plugins: Vec<Box<dyn Plugin>>) -> Self {
        PluginManager {
            plugins,
            lua_plugins: Vec::new(),
            lua_hook_queue: RefCell::new(Vec::new()),
        }
    }

    pub fn enqueue_lua_hooks(&self, hook_name: &str) {
        let mut q = self.lua_hook_queue.borrow_mut();
        for (i, _) in self.lua_plugins.iter().enumerate() {
            q.push((i, hook_name.to_string()));
        }
    }

    pub fn flush_lua_hooks(&self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        let mut q = self.lua_hook_queue.borrow_mut();
        let queued = std::mem::take(&mut *q);
        drop(q);

        for (plugin_idx, hook_name) in queued {
            let plugin = &self.lua_plugins[plugin_idx];
            let globals = plugin.lua_vm.globals();
            let snapshot = create_snapshot(cpu, &plugin.plugin_name);

            globals
                .set("cpu", snapshot.clone())
                .map_err(|e| EmuError::PluginError {
                    message: format!("[PluginManager][Lua] Error injecting snapshot: {}", e),
                })?;

            if let Some(fn_name) = plugin.hook_map.get(&hook_name) {
                if let Ok(func) = globals.get::<mlua::Function>(fn_name.as_str()) {
                    if let Err(e) = func.call::<()>(()) {
                        return Err(EmuError::PluginError {
                            message: format!(
                                "[PluginManager][Lua][{}] Error calling '{}' hook: {}",
                                plugin.plugin_name, fn_name, e
                            ),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub fn load_lua_plugin(&mut self, file_path: &str, plugin_name: String) -> EmuResult<()> {
        let script_code = fs::read_to_string(file_path).map_err(|e| {
            // EmuError::IoError(format!("Failed to read Lua script {}: {}", file_path, e))
            EmuError::FileError {
                path: Some(file_path.to_string()),
                message: format!("Failed to read Lua script {}: {}", file_path, e),
            }
        })?;
        let lua = Lua::new();
        initialize_lua_environment(&lua).map_err(|e| EmuError::PluginError {
            message: format!("Lua env init failed: {}", e),
        })?;
        lua.load(&script_code)
            .exec()
            .map_err(|e| EmuError::PluginError {
                message: format!("Lua initial execution failed: {}", e),
            })?;

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
            plugin_name: plugin_name.clone(),
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

    pub fn run_hooks(&mut self, hook_name: &str, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.enqueue_lua_hooks(hook_name);
        self.flush_lua_hooks(cpu)?;
        for plugin in &mut self.plugins {
            let snapshot = create_snapshot(cpu, plugin._name());
            let mem = crate::memory::Memory::from_ram(snapshot.memory.clone());
            let regs = &snapshot.registers;
            match hook_name {
                "on_plugin_load" => plugin.on_plugin_load(regs, &mem)?,
                "on_plugin_unload" => plugin.on_plugin_unload(regs, &mem)?,
                "pre_pc_increment" => {
                    let _ = plugin.pre_pc_increment(regs, &mem)?;
                }
                "post_pc_increment" => plugin.post_pc_increment(regs, &mem)?,
                "pre_syscall" => {
                    let _ = plugin.pre_syscall(regs, &mem, cpu.get_reg(8))?;
                }
                "post_syscall" => plugin.post_syscall(regs, &mem, cpu.get_reg(8))?,
                "pre_bl" => {
                    let _ = plugin.pre_bl(regs, &mem, cpu.get_reg(30))?;
                }
                "post_bl" => plugin.post_bl(regs, &mem, cpu.get_reg(30))?,
                "pre_ret" => {
                    let _ = plugin.pre_ret(regs, &mem)?;
                }
                "post_ret" => plugin.post_ret(regs, &mem)?,
                _ => (),
            }
        }
        Ok(())
    }

    // ---- ONE-TO-ONE HOOKS, NO SKIP ----
    pub fn pre_pc_increment(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("pre_pc_increment", cpu)
    }
    pub fn post_pc_increment(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("post_pc_increment", cpu)
    }
    pub fn pre_syscall(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("pre_syscall", cpu)
    }
    pub fn post_syscall(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("post_syscall", cpu)
    }
    pub fn pre_bl(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("pre_bl", cpu)
    }
    pub fn post_bl(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("post_bl", cpu)
    }
    pub fn pre_ret(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("pre_ret", cpu)
    }
    pub fn post_ret(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("post_ret", cpu)
    }
    pub fn on_plugin_load(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("on_plugin_load", cpu)
    }
    pub fn on_plugin_unload(&mut self, cpu: &crate::cpu::CpuState) -> EmuResult<()> {
        self.run_hooks("on_plugin_unload", cpu)
    }
}
