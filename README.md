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

### Prebuilt Binaries
Prebuilt binaries are available on the [Releases](https://github.com/aschombe/aarch64-emulator/releases) page.  
There is only one download for Windows and its x86/amd64.  
If you are on macOS:  
- How to check if you have an Intel or Apple Silicon Mac:  
  ```bash
  uname -m
  ```
- For Intel Macs, download the x86/amd64 build.  
- For Apple Silicon Macs, download the aarch64/arm64 build.  
- If you are on Linux, I bet you already know which one to download.  

### From Source
Requires:
- Rust 1.90+ with Cargo  
- Linux or macOS (Haven't tested on Windows yet)

Build and run from source:
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
  -v, --verbose           Enables verbose execution tracing and syscall debug messages in run mode  
  -d, --debug             Enables the GDB-like interactive TUI debugger. Cannot be used with plugins  
  -p, --plugins <files>   Comma-separated list of Lua plugin file paths to load. Cannot be used with debug  
  -f, --filesystem <path> Folder path to give the emulated program access to the files within. Mounts to VFS root ('/')  
  -e, --entry <label>     Specify a custom entry point label [default: _start]  
  -h, --help              Print help information  
  -V, --version           Print version information  

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
- [ ] Use AWS Lambda and API gateway to provide an online emulator service
    - HTML/CSS/JS frontend that allows users to upload assembly files or select options
    - Backend runs the emulator and streams output back to the frontend
    - How will plugins or the filesystem or debugger work in this scenario?
- [ ] SIMD/FP support
- [ ] SME and SVE support?
- [ ] Memory segment/boundary checks/enforcement (no writing to code segments, etc.)
- [ ] Allow labels to start with '.' (currently reserved for directives)
- Do you have to explicitly define a label as extern to call it from another file?
- Allow the use of real c library/other library functions?

**Instructions**:
- https://developer.arm.com/documentation/ddi0487/latest:
    - ADD SUPPORT FOR SHIFT AND EXTEND MODIFIERS WHERE APPLICABLE:
        - Instructions that support these:
            - MOVN/Z/K (MOVN/Z/K <Wd/Xd>, {#}<imm>{, LSL #<shift>})
            - NEG/S (NEG/S <Wd/Xd>, <Wn/Xn>{, <shift> #<amount>}) (double check NEGS)
            - ADD (Extended Register) (ADD <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <extend> {#<amount>}})
            - ADD (Immediate) (ADD <Wd/Xd>, <Wn/Xn>, #<imm>{, <shift>})
            - ADD (Shifted Register) (ADD <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - ADDS (Extended Register) (ADDS <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <extend> {#<amount>}})
            - ADDS (Immediate) (ADDS <Wd/Xd>, <Wn/Xn>, #<imm>{, <shift>})
            - ADDS (Shifted Register) (ADDS <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - SUB (Extended Register) (SUB <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <extend> {#<amount>}})
            - SUB (Immediate) (SUB <Wd/Xd>, <Wn/Xd>, #<imm>{, <shift>})
            - SUB (Shifted Register) (SUB <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - SUBS (Extended Register) (SUBS <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <extend> {#<amount>}})
            - SUBS (Immediate) (SUBS <Wd/Xd>, <Wn/Xn>, #<imm>{, <shift>})
            - SUBS (Shifted Register) (SUBS <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})

            - DOUBLE CHECK SYNTAX ON THESE --------
            - AND (Shifted Register) (AND <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - ORR (Shifted Register) (ORR <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - EOR (Shifted Register) (EOR <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - ANDS (Shifted Register) (ANDS <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - BIC (Shifted Register) (BIC <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - BICS (Shifted Register) (BICS <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - EON (Shifted Register) (EON <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - ORN (Shifted Register) (ORN <Wd/Xd>, <Wn/Xn>, <Rm/Xm>{, <shift> #<amount>})
            - TST (Shifted Register) (TST <Wd/Xd>, <Wn/Xn>{, <shift> #<amount>})
            - -------------

            - ASR (Immediate) (ASR <Wd/Xd>, <Wn/Xn>, #<shift>)
            - ROR (Immediate) (ROR <Wd/Xd>, <Wn/Xn>, #<shift>)
            - LSR (Immediate) (LSR <Wd/Xd>, <Wn/Xn>, #<shift>)
            - LSL (Immediate) (LSL <Wd/Xd>, <Wn/Xn>, #<shift>)

            - LDR (Register) (LDR <Xt>, [<Xn|SP>, (<Wm>|<Xm>){, <extend> {#<amount>}}])
            - LDRB (Register Extended) (LDRB <Wt>, [<Xn|SP>, (<Wm>|<Xm>), <extend> {<amount>}])
            - LDRB (Register Shifted) (LDRB <Wt>, [<Xn|SP>, <Xm>{, LSL <amount>}])
            - LDRH (Reigster) (LDRH <Wt>, [<Xn|SP>, (<Wm>|<Xm>){, <extend> {<amount>}}])
            - LDRSB (Register Extended) (LDRSB <Xt>, [<Xn|SP>, (<Wm>|<Xm>), <extend> {<amount>}])
            - LDRSB (Register Shifted) (LDRSB <Xt>, [<Xn|SP>, <Xm>{, LSL <amount>}])
            - LDRSH (Register) (LDRSH <Xt>, [<Xn|SP>, (<Wm>|<Xm>){, <extend> {<amount>}}])
            - LDRSW (Register) (LDRSW <Xt>, [<Xn|SP>, (<Wm>|<Xm>){, <extend> {<amount>}}])
            - STR (Register) (STR <Xt>, [<Xn|SP>, (<Wm>|<Xm>){, <extend> {<amount>}}])
            - STRB (Register Extended) (STRB <Wt>, [<Xn|SP>, (<Wm>|<Xm>), <extend> {<amount>}])
            - STRB (Register Shifted) (STRB <Wt>, [<Xn|SP>, <Xm>{, LSL <amount>}])
            - STRH (Register) (STRH <Wt>, [<Xn|SP>, (<Wm>|<Xm>){, <extend> {<amount>}}])

            - CMN (Extended Register) (CMN <Xn|SP>, <R><m>{, <extend> {#<amount>}})
            - CMN (Immediate) (CMN <Xn|SP>, #<imm>{, <shift>})
            - CMN (Shifted Register) (CMN <Xn>, <Xm>{, <shift> #<amount>})
            - CMP (Extended Register) (CMP <Xn|SP>, <R><m>{, <extend> {#<amount>}})
            - CMP (Immediate) (CMP <Xn|SP>, #<imm>{, <shift>})
            - CMP (Shifted Register) (CMP <Xn>, <Xm>{, <shift> #<amount>})
            - TST (Shifted Register) (TST <Xn>, <Xm>{, <shift> #<amount>})

            - STRB (Register) (Shifted Register)
            - STRB (Register) (Extended Register)
            - LDRB (Register) (Shifted Register)
            - LDRB (Register) (Extended Register)
            - LDRSB (Register) (Shifted register)
            - LDRSB (Register) (Extended register)

        - Modifiers:
            - LSL, LSR, ASR, ROR
            - UXTB, UXTH, UXTW, UXTX
            - SXTB, SXTH, SXTW, SXTX
    - Address (C1.3):
        - ldr immediates? (ldr xn, xm where xm is the adr of a variable)


- https://www.cs.princeton.edu/courses/archive/fall19/cos217/reading/ArmInstructionSetOverview.pdf
- Test negative offsets with ldr/str instructions
- https://developer.arm.com/documentation/ddi0602/2025-09/Base-Instructions
- [ ] Add directives:
    - Readd and DEBUG .rept and .endr

**Virtual File System**:
- [ ] Come up with more things to add

**Syscall Layer**:
- [ ] Expand syscall coverage (see syscalls.txt)

**Debugger (TUI)**:
- [ ] Fix delayed instruction indicator
- [ ] Synchronize highlight scrolling

**Plugins**:
- [ ] Allow pre and post svc hooks know the syscall number
- [ ] Test calling convention (CC) plugin
- [ ] Add more hooks as needed
- [ ] Add more API functions to Lua CPU object

**Documentation**:
- [ ] Expand inline comments and Rustdocs

---

## License

MIT License — simple, permissive, and widely used.  
You are free to copy, modify, and distribute this software with attribution.

See the [LICENSE](LICENSE) file for full text.
