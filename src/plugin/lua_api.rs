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
        methods.add_method("get_reg", |_, this, id: i64| {
            if !(0..=32).contains(&id) {
                return Err(mlua::Error::runtime(format!("Invalid register ID: {}", id)));
            }

            let reg_val = this.state.borrow().get_reg(id as usize);

            Ok(reg_val as Word)
        });

        methods.add_method(
            "get_ip",
            |_, this, _: ()| Ok(this.state.borrow().ip as Word),
        );

        methods.add_method("get_pstate", |_, this, _: ()| {
            Ok(this.state.borrow().pstate)
        });

        methods.add_method("peek_word", |_, this, addr: Word| {
            match this.state.borrow().memory.read_word(addr) {
                Ok(val) => Ok(val),
                Err(e) => Err(mlua::Error::runtime(format!("Memory read error: {}", e))),
            }
        });

        methods.add_method(
            "peek_bytes",
            |lua, this, (addr, len): (Word, usize)| match this
                .state
                .borrow()
                .memory
                .read_bytes(addr, len)
            {
                Ok(bytes) => {
                    let lua_table = lua.create_table()?;
                    for (i, byte) in bytes.iter().enumerate() {
                        lua_table.set(i + 1, *byte)?;
                    }
                    Ok(Value::Table(lua_table))
                }
                Err(e) => Err(mlua::Error::runtime(format!("Memory read error: {}", e))),
            },
        );

        methods.add_method("log", |_, this, msg: String| {
            // println!("[LUA] {}", msg);
            // instead of "LUA" use the name of the plugin
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
