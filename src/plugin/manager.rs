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
use mlua::{Function, Lua, Value};

/// Maps a hook name (e.g., "pre_exec") to a script function name (e.g., "on_pre_execution").
type ScriptHookMap = HashMap<String, String>;

/// Represents a loaded Lua script plugin with its own Lua instance and context.
pub struct LuaPlugin {
    pub lua_vm: Lua,
    pub lua_ctx: LuaContext,
    pub _script_code: String,
    pub hook_map: ScriptHookMap,
}

pub struct PluginManager {
    pub plugins: Vec<Box<dyn Plugin>>,
    pub lua_plugins: Vec<LuaPlugin>,
}

impl PluginManager {
    pub fn new(plugins: Vec<Box<dyn Plugin>>) -> Self {
        PluginManager {
            plugins,
            lua_plugins: Vec::new(),
        }
    }

    /// Loads a Lua script, creates its own Lua VM and context with the plugin name, and registers hooks.
    pub fn load_lua_plugin(
        &mut self,
        file_path: &str,
        cpu_state: Rc<RefCell<crate::cpu::CpuState>>,
        plugin_name: String,
    ) -> EmuResult<()> {
        let script_code = fs::read_to_string(file_path).map_err(|e| {
            EmuError::IoError(format!("Failed to read Lua script {}: {}", file_path, e))
        })?;

        // Create a new Lua instance for this plugin
        let lua = Lua::new();
        initialize_lua_environment(&lua)
            .map_err(|e| EmuError::InternalError(format!("Lua env init failed: {}", e)))?;

        // Create the LuaContext with the plugin_name for logging
        let lua_ctx = LuaContext::new(cpu_state.clone(), plugin_name.clone());

        // Register cpu userdata in this Lua VM's globals
        let globals = lua.globals();
        globals
            .set("cpu", lua.create_userdata(lua_ctx.clone())?)
            .map_err(|e| EmuError::InternalError(format!("Setting cpu userdata failed: {}", e)))?;

        // Execute the script immediately to define global functions
        lua.load(&script_code)
            .exec()
            .map_err(|e| EmuError::InternalError(format!("Lua initial execution failed: {}", e)))?;

        // Construct hook map - adjust accordingly
        let hook_map = HashMap::from([
            ("on_plugin_load".to_string(), "on_plugin_load".to_string()),
            (
                "on_plugin_unload".to_string(),
                "on_plugin_unload".to_string(),
            ),
            ("pre_execution_event".to_string(), "on_pre_exec".to_string()),
            (
                "post_execution_event".to_string(),
                "on_post_exec".to_string(),
            ),
            (
                "pre_syscall_execution".to_string(),
                "on_pre_syscall".to_string(),
            ),
            (
                "post_syscall_execution".to_string(),
                "on_post_syscall".to_string(),
            ),
        ]);

        self.lua_plugins.push(LuaPlugin {
            lua_vm: lua,
            lua_ctx,
            _script_code: script_code,
            hook_map,
        });

        if VERBOSE_ENABLED.load(Ordering::Relaxed) {
            println!(
                "[PluginManager] Loaded Lua script plugin '{}' from {}.",
                plugin_name, file_path
            );
        }
        Ok(())
    }

    /// Executes Lua hook function across all loaded Lua plugins.
    pub fn execute_lua_hook(
        &mut self,
        _cpu_regs: &[Word; 32],
        _memory: memory::Memory,
        hook_name: &str,
    ) -> EmuResult<bool> {
        let mut handled = false;

        if VERBOSE_ENABLED.load(Ordering::Relaxed) {
            println!("[PluginManager] Executing Lua hook '{}'", hook_name);
        }

        for plugin in &mut self.lua_plugins {
            if let Some(fn_name) = plugin.hook_map.get(hook_name) {
                let globals = plugin.lua_vm.globals();

                match globals.get::<Function>(fn_name.as_str()) {
                    Ok(func) => match func.call(()) {
                        Ok(result) => {
                            if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                                println!(
                                    "[PluginManager][{}] Lua hook '{}' executed successfully.",
                                    plugin.lua_ctx.plugin_name, fn_name
                                );
                            }
                            if let Value::Boolean(true) = result {
                                handled = true;
                            }
                        }
                        Err(e) => {
                            return Err(EmuError::InternalError(format!(
                                "[PluginManager][{}] Error calling Lua hook '{}': {}",
                                plugin.lua_ctx.plugin_name, fn_name, e
                            )));
                        }
                    },
                    Err(mlua::Error::FromLuaConversionError { from, .. }) if from == "nil" => {
                        if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                            println!(
                                "[PluginManager][{}] Lua function '{}' is nil (not defined), skipping.",
                                plugin.lua_ctx.plugin_name, fn_name
                            );
                        }
                        continue;
                    }
                    Err(e) => {
                        return Err(EmuError::InternalError(format!(
                            "[PluginManager][{}] Error getting Lua function '{}': {}",
                            plugin.lua_ctx.plugin_name, fn_name, e
                        )));
                    }
                }
            } else if VERBOSE_ENABLED.load(Ordering::Relaxed) {
                println!(
                    "[PluginManager] No Lua function mapped for hook '{}'. Skipping.",
                    hook_name
                );
            }
        }

        Ok(handled)
    }

    pub fn on_plugin_load(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
    ) -> EmuResult<()> {
        self.execute_lua_hook(cpu_regs, memory.clone(), "on_plugin_load")?;
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
        self.execute_lua_hook(cpu_regs, memory.clone(), "on_plugin_unload")?;
        for plugin in &mut self.plugins {
            plugin.on_plugin_unload(cpu_regs, &memory)?;
        }
        Ok(())
    }

    pub fn pre_execution_event(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
    ) -> EmuResult<bool> {
        let mut should_skip =
            self.execute_lua_hook(cpu_regs, memory.clone(), "pre_execution_event")?;

        for plugin in &mut self.plugins {
            if plugin.pre_execution_event(cpu_regs, &memory)? {
                should_skip = true;
            }
        }
        Ok(should_skip)
    }

    pub fn post_execution_event(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
    ) -> EmuResult<()> {
        self.execute_lua_hook(cpu_regs, memory.clone(), "post_execution_event")?;
        for plugin in &mut self.plugins {
            plugin.post_execution_event(cpu_regs, &memory)?;
        }
        Ok(())
    }

    pub fn pre_syscall_execution(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
        sys_call_num: Word,
    ) -> EmuResult<bool> {
        let mut should_skip =
            self.execute_lua_hook(cpu_regs, memory.clone(), "pre_syscall_execution")?;
        for plugin in &mut self.plugins {
            if plugin.pre_syscall_execution(cpu_regs, &memory, sys_call_num)? {
                should_skip = true;
            }
        }
        Ok(should_skip)
    }

    pub fn post_syscall_execution(
        &mut self,
        cpu_regs: &[Word; 32],
        memory: memory::Memory,
        sys_call_num: Word,
    ) -> EmuResult<()> {
        self.execute_lua_hook(cpu_regs, memory.clone(), "post_syscall_execution")?;
        for plugin in &mut self.plugins {
            plugin.post_syscall_execution(cpu_regs, &memory, sys_call_num)?;
        }
        Ok(())
    }
}
