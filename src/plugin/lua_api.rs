// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::cpu::CpuState;
use crate::types::Word;
use mlua::{Lua, Result as LuaResult, UserData, UserDataMethods, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// Lua Context that wraps the CpuState in Rc<RefCell<_>>
#[derive(Clone)]
pub struct LuaContext {
    pub state: Rc<RefCell<CpuState>>,
    pub plugin_name: String,
}

impl LuaContext {
    pub fn new(state: Rc<RefCell<CpuState>>, plugin_name: String) -> Self {
        LuaContext { state, plugin_name }
    }
}

impl UserData for LuaContext {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // Safe getter for registers (short borrow)
        methods.add_method("get_reg", |_, this, id: i64| {
            if !(0..=32).contains(&id) {
                return Err(mlua::Error::runtime(format!("Invalid register ID: {}", id)));
            }

            let val = {
                let Ok(state_ref) = this.state.try_borrow() else {
                    return Err(mlua::Error::runtime("CPU already mutably borrowed"));
                };
                state_ref.get_reg(id as usize)
            };
            Ok(val as Word)
        });

        // Instruction pointer
        methods.add_method("get_ip", |_, this, _: ()| {
            let Ok(state_ref) = this.state.try_borrow() else {
                return Err(mlua::Error::runtime("CPU already mutably borrowed"));
            };
            Ok(*state_ref.ip.borrow() as Word)
        });

        // Processor state flags
        methods.add_method("get_pstate", |_, this, _: ()| {
            let Ok(state_ref) = this.state.try_borrow() else {
                return Err(mlua::Error::runtime("CPU already mutably borrowed"));
            };
            Ok(*state_ref.pstate.borrow())
        });

        // Read a 64-bit word from memory
        methods.add_method("peek_word", |_, this, addr: Word| {
            let Ok(state_ref) = this.state.try_borrow() else {
                return Err(mlua::Error::runtime("CPU already mutably borrowed"));
            };
            let mem_ref = state_ref.memory.borrow();
            match mem_ref.read_word(addr) {
                Ok(val) => Ok(val),
                Err(e) => Err(mlua::Error::runtime(format!("Memory read error: {}", e))),
            }
        });

        // Read bytes from memory
        methods.add_method("peek_bytes", |lua, this, (addr, len): (Word, usize)| {
            let Ok(state_ref) = this.state.try_borrow() else {
                return Err(mlua::Error::runtime("CPU already mutably borrowed"));
            };
            let mem_ref = state_ref.memory.borrow();
            match mem_ref.read_bytes(addr, len) {
                Ok(bytes) => {
                    let lua_table = lua.create_table()?;
                    for (i, b) in bytes.iter().enumerate() {
                        lua_table.set(i + 1, *b)?;
                    }
                    Ok(Value::Table(lua_table))
                }
                Err(e) => Err(mlua::Error::runtime(format!("Memory read error: {}", e))),
            }
        });

        // Host-side log
        methods.add_method("log", |_, this, msg: String| {
            println!("[{}] {}", this.plugin_name, msg);
            Ok(())
        });
    }
}

pub fn initialize_lua_environment(lua: &Lua) -> LuaResult<()> {
    let globals = lua.globals();
    globals.set(
        "print_host",
        lua.create_function(|_, msg: String| {
            println!("[LUA] {}", msg);
            Ok(())
        })?,
    )?;
    Ok(())
}
