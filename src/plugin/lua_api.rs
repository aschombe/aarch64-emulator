use crate::cpu::CpuState;
use crate::types::Word;
use mlua::{Lua, Result as LuaResult, UserData, UserDataMethods, Value};

#[derive(Clone)]
pub struct LuaCpuSnapshot {
    pub registers: [Word; 32],
    pub sp: Word,
    pub ip: Word,
    pub cpsr: Word,
    pub memory: Vec<u8>,
    pub plugin_name: String,
}

impl UserData for LuaCpuSnapshot {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_reg", |_, this, id: i64| {
            if !(0..=32).contains(&id) {
                return Err(mlua::Error::runtime(format!("Invalid register ID: {}", id)));
            }
            if id == 32 {
                Ok(this.sp)
            } else {
                Ok(this.registers[id as usize])
            }
        });
        methods.add_method("get_ip", |_, this, _: ()| Ok(this.ip));
        methods.add_method("get_cpsr", |_, this, _: ()| Ok(this.cpsr));
        methods.add_method("log", |_, this, msg: String| {
            println!("[{}] {}", this.plugin_name, msg);
            Ok(())
        });
        // Peek word from memory snapshot
        methods.add_method("peek_word", |_, this, addr: Word| {
            let base = addr as usize;
            if base + 8 > this.memory.len() {
                return Err(mlua::Error::runtime(format!(
                    "Memory read OOB at 0x{:x}",
                    addr
                )));
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&this.memory[base..base + 8]);
            Ok(Word::from_le_bytes(bytes))
        });
        methods.add_method("peek_bytes", |lua, this, (addr, len): (Word, usize)| {
            let base = addr as usize;
            if base + len > this.memory.len() {
                return Err(mlua::Error::runtime(format!(
                    "Memory read OOB at 0x{:x} (len {})",
                    addr, len
                )));
            }
            let bytes = &this.memory[base..base + len];
            let lua_table = lua.create_table()?;
            for (i, b) in bytes.iter().enumerate() {
                lua_table.set(i + 1, *b)?;
            }
            Ok(Value::Table(lua_table))
        });
    }
}

pub fn create_snapshot(cpu: &CpuState, plugin_name: &str) -> LuaCpuSnapshot {
    let memory = cpu.memory.borrow().ram.clone();
    LuaCpuSnapshot {
        registers: *cpu.registers.borrow(),
        sp: *cpu.sp.borrow(),
        ip: *cpu.ip.borrow() as Word,
        cpsr: *cpu.cpsr.borrow(),
        memory,
        plugin_name: plugin_name.to_string(),
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
