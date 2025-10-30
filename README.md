# aarch64-emulator

A modular and extensible AArch64 emulator written in Rust.  

It features a modular CPU core, assembler, Linux syscall emulation, Ratatui-based TUI debugger, and Lua-based plugin system with lifecycle and execution hooks.

---

## Features

- Basic AArch64 CPU interpreter
- Ratatui-based debugger with source, register, and memory views
- Virtual memory and virtual filesystem (VFS)
- Partial Linux syscall emulation
- Lua plugin support with lifecycle and execution hooks

---

## Installation

Requires:
- Rust 1.90+ with Cargo  
- Linux or macOS (Haven't tested on Windows yet)

Build and run:
```bash
git clone https://github.com/aschombe/aarch64-emulator.git
cd aarch64-emulator
cargo build --release
cargo run -- --help
```

---

## Usage

    cargo run -- [OPTIONS] <input_file>

Options:  
  -d, --debug             Start TUI debugger  
  --verbose               Enable verbose output and tracing  
  --plugins <files>       Comma-separated Lua plugin file paths  
  --filesystem <path>     Mount a host folder as a virtual filesystem  

Example:  
    cargo run -- --debug --plugins ./plugins/logger.lua examples/hello_world.s  
    cargo run -- --filesystem /tmp examples/file_io.s # This mounts /tmp as the VFS root  

---

## Architecture Overview

Core modules:  
  assembler    - Parses source, resolves symbols  
  cpu          - Execution pipeline and register state  
  memory       - Virtual address space and safety checks  
  syscall      - Linux syscall emulation  
  debugger     - TUI powered by ratatui  
  vfs          - Virtual file system abstraction  
  plugin       - Lua and Rust plugin architecture  
  types.rs     - Core constants, types, and errors  

---

## Plugin Interface

The emulator supports **both Lua and Rust plugins** for runtime extension.

Lua plugin hooks are automatically discovered by the PluginManager and executed at the right points in the emulation lifecycle.

### Plugin Lifecycle Hooks

- on_plugin_load() — called once after plugin load  
- on_plugin_unload() — called once at shutdown

### Execution Hooks

- pre_pc_increment() / post_pc_increment()
- pre_syscall() / post_syscall()
- pre_bl(target_addr) / post_bl(target_addr)
- pre_ret() / post_ret()

### Example Lua Plugin

    function on_plugin_load()
        print("Initialized successfully!")
    end

    function pre_syscall()
        print("Syscall encountered")
    end

    function on_plugin_unload()
        print("Unloaded safely")
    end

Each Lua plugin runs in its own VM, isolated but sharing access to the emulator state via the CPU context.

### Lua CPU API

Available functions (via `cpu` object):  
    cpu:get_reg(id)          -- Get register value (0-32), 32 is SP  
    cpu:get_ip()             -- Get instruction pointer  
    cpu:get_pstate()         -- Get processor state flags  
    cpu:peek_word(addr)      -- Read 64-bit word from memory  
    cpu:peek_bytes(addr, len) -- Read a sequence of bytes from memory  
    cpu:log(msg)              -- Log message to host console (plugin prefix is automatically added)  

Example command to run with multiple plugins:  
    cargo run -- --plugins="./path/to/plugin.lua","./path/to/another.lua" examples/test.s  

---

## TODO

**Architecture**:
- [ ] SIMD/FP support
- [ ] Memory segment/boundary checks/enforcement (no writing to code segments, etc.)

**Instructions**:
- [ ] Add more AArch64 instruction support
    - Make sure ADRP, LDP, STP, LDR= [example](./examples/cheater.s) is working
    - https://developer.arm.com/documentation/ddi0602/2025-09/Base-Instructions
- [ ] Add directives:
    - .byte, .single/.float, .double
    - .balign 
    - .rept and .endr
    - .extern

**Virtual File System**:
- [ ] Test file I/O (open/read/write/lseek)
- [ ] Validate flag and permission mapping

**Syscall Layer**:
- [ ] Expand syscall coverage (see syscalls.txt)

**Debugger (TUI)**:
- [ ] Fix delayed instruction indicator
- [ ] Synchronize highlight scrolling

**Plugins**:
- [ ] Add and test calling convention (CC) plugin
- [ ] Add more hooks as needed
- [ ] Add more API functions to Lua CPU object

**Documentation**:
- [ ] Expand inline comments and Rustdocs

---

## License

MIT License — simple, permissive, and widely used.  
You are free to copy, modify, and distribute this software with attribution.

See the [LICENSE](LICENSE) file for full text.
