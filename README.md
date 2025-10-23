# Aarch64 Emulator in Rust

## Todo:
- [ ] Plugins:
    - [ ] Implement CC checker
- [ ] Psuedo Filesystem:
    - [ ] Accept a folder as a mount in the emulator flags
    - [ ] That folder's contents are accessible to the program at / 
    - [ ] Implement I/O or file syscalls 
- [ ] Syscall:
    - [ ] Bins and dotprod not printing properly
    - [ ] Add more syscalls
        - [ ] Debug VFS syscalls
        - [ ] Check [syscall list](./syscalls.txt) for more ideas
- [ ] TUI debugger:
    - [ ] The instruction indicator delays by one cycle after skipping whitespace
    - [ ] The instruction highlighter (for scrolling) is 2 instructions behind the instruction indicator
- [ ] DOCUMENTATION!!!
- [ ] Add more instructions
