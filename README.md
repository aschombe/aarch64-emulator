# Aarch64 Emulator in Rust

## Todo:
- [ ] Core:
    - [ ] W register vs X register handling?
- [ ] Plugins:
    - [ ] Test CC checker
- [ ] Psuedo Filesystem:
    - [ ] Test filesystem and I/O syscalls thoroughly
- [ ] Syscall:
    - [ ] atoi, bins, dotprod not printing properly (write syscall issue, or escape_string)
    - [ ] display32 infinite loop
    - [ ] Add more syscalls
        - [ ] Check [syscall list](./syscalls.txt) for more ideas
- [ ] TUI debugger:
    - [ ] The TUI renders all data sections in the source code viewer and its ugly and hard to understand 
    - [ ] The instruction indicator delays by one cycle after skipping whitespace
    - [ ] The instruction highlighter (for scrolling) is 2 instructions behind the instruction indicator
- [ ] DOCUMENTATION!!!
- [ ] Add more instructions
