// use std::cell::RefCell;
// use std::collections::HashMap;
// use std::fs;
// use std::rc::Rc;
// use std::sync::atomic::Ordering;
// use std::vec::Vec;
//
// use super::Plugin;
// use super::lua_api::{LuaContext, initialize_lua_environment};
// use crate::cpu::CpuState;
// use crate::types::{EmuError, EmuResult, VERBOSE_ENABLED, Word};
// use mlua::{Chunk, Function, Lua, Result as LuaResult, Value};
//
// /// Maps a hook name (e.g., "pre_exec") to a script function name (e.g., "on_pre_execution").
// type ScriptHookMap = HashMap<String, String>;
//
// /// Represents a loaded Lua script plugin.
// struct LuaPlugin {
//     pub script_code: String,
//     pub hook_map: ScriptHookMap,
// }
//
// pub struct PluginManager {
//     pub plugins: Vec<Box<dyn Plugin>>,
//     pub lua_vm: Lua,
//     pub lua_plugins: Vec<LuaPlugin>,
//     pub lua_ctx: Option<LuaContext>,
// }
//
// impl PluginManager {
//     pub fn new() -> Self {
//         let lua = Lua::new();
//         initialize_lua_environment(&lua).expect("Failed to initialize Lua environment.");
//
//         PluginManager {
//             plugins: Vec::new(),
//             lua_vm: lua,
//             lua_plugins: Vec::new(),
//             lua_ctx: None,
//         }
//     }
//
//     /// Initializes the Lua context with the mutable CPU state reference.
//     pub fn init_lua(&mut self, cpu_state: Rc<RefCell<crate::cpu::CpuState>>) -> mlua::Result<()> {
//         let context = LuaContext::new(cpu_state.clone());
//         let globals = self.lua_vm.globals();
//
//         globals.set("cpu", self.lua_vm.create_userdata(context.clone())?)?;
//         self.lua_ctx = Some(context);
//
//         Ok(())
//     }
//
//     /// Loads a static plugin instance using the standard Plugin trait.
//     pub fn load_plugin(&mut self, plugin: Box<dyn Plugin>) {
//         // plugin.on_plugin_load();
//         self.plugins.push(plugin);
//     }
//
//     /// Loads a Lua script, compiles it, and registers the global hooks.
//     pub fn load_lua_plugin(&mut self, file_path: &str) -> EmuResult<()> {
//         let script_code = fs::read_to_string(file_path).map_err(|e| {
//             EmuError::IoError(format!("Failed to read Lua script {}: {}", file_path, e))
//         })?;
//
//         // Execute the script immediately to define global functions.
//         self.lua_vm
//             .load(&script_code)
//             .exec()
//             .map_err(|e| EmuError::InternalError(format!("Lua initial execution failed: {}", e)))?;
//
//         // Mock Hook Map for demonstration purposes.
//         let hook_map = HashMap::from([
//             ("pre_execution_event".to_string(), "on_pre_exec".to_string()),
//             (
//                 "post_execution_event".to_string(),
//                 "on_post_exec".to_string(),
//             ),
//             (
//                 "pre_syscall_execution".to_string(),
//                 "on_pre_syscall".to_string(),
//             ),
//             (
//                 "post_syscall_execution".to_string(),
//                 "on_post_syscall".to_string(),
//             ),
//         ]);
//
//         self.lua_plugins.push(LuaPlugin {
//             script_code,
//             hook_map,
//         });
//
//         if VERBOSE_ENABLED.load(Ordering::Relaxed) {
//             println!("[Manager] Loaded Lua script plugin from {}.", file_path);
//         }
//         Ok(())
//     }
//
//     /// Executes a single Lua hook function across all plugins.
//     fn execute_lua_hook(&mut self, state: &mut CpuState, hook_name: &str) -> EmuResult<bool> {
//         let mut handled = false;
//
//         for plugin in &self.lua_plugins {
//             if let Some(fn_name) = plugin.hook_map.get(hook_name) {
//                 // 1. Get the function from the global scope.
//                 let globals = self.lua_vm.globals();
//
//                 let hook_fn: Function = match globals.get(fn_name.as_str()) {
//                     Ok(f) => f,
//                     Err(_) => continue,
//                 };
//
//                 // 2. Call the function. We rely on the global 'cpu' object being set up.
//
//                 // if let Ok(Value::Boolean(b)) = result {
//                 //     if b {
//                 //         handled = true;
//                 //     }
//                 // } else if let Err(e) = result {
//                 //     eprintln!("[LUA ERROR] Plugin hook '{}' failed: {}", fn_name, e);
//                 // }
//             }
//         }
//
//         Ok(handled)
//     }
//
//     pub fn on_plugin_load(&mut self) -> EmuResult<()> {
//         for plugin in &mut self.plugins {
//             plugin.on_plugin_load()?;
//         }
//         Ok(())
//     }
//
//     pub fn on_plugin_unload(&mut self) -> EmuResult<()> {
//         for plugin in &mut self.plugins {
//             plugin.on_plugin_unload()?;
//         }
//         Ok(())
//     }
//
//     pub fn pre_execution_event(&mut self, state: &mut CpuState) -> EmuResult<bool> {
//         let mut should_skip = self.execute_lua_hook(state, "pre_execution_event")?;
//         for plugin in &mut self.plugins {
//             if plugin.pre_execution_event(state)? {
//                 should_skip = true;
//             }
//         }
//         Ok(should_skip)
//     }
//
//     pub fn post_execution_event(&mut self, state: &mut CpuState) -> EmuResult<()> {
//         self.execute_lua_hook(state, "post_execution_event")?;
//         for plugin in &mut self.plugins {
//             plugin.post_execution_event(state)?;
//         }
//         Ok(())
//     }
//
//     pub fn pre_syscall_execution(
//         &mut self,
//         state: &mut CpuState,
//         sys_call_num: Word,
//     ) -> EmuResult<bool> {
//         let mut should_skip = self.execute_lua_hook(state, "pre_syscall_execution")?;
//         for plugin in &mut self.plugins {
//             if plugin.pre_syscall_execution(state, sys_call_num)? {
//                 should_skip = true;
//             }
//         }
//         Ok(should_skip)
//     }
//
//     pub fn post_syscall_execution(
//         &mut self,
//         state: &mut CpuState,
//         sys_call_num: Word,
//     ) -> EmuResult<()> {
//         self.execute_lua_hook(state, "post_syscall_execution")?;
//         for plugin in &mut self.plugins {
//             plugin.post_syscall_execution(state, sys_call_num)?;
//         }
//         Ok(())
//     }
// }
