# Supported Syscalls

## General
- Syscalls are invoked via:
    - x8 = syscall number
    - x0-x7 = arguments (extra arguments passed on stack)
    - svc 0
    - return value in x0

## File Operations
- openat:
    - x0 = dirfd
    - x1 = pointer to pathname
    - x2 = flags (O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_TRUNC, O_APPEND)
    - x3 = currently unused
    - x8 = 56
    - On success, returns file descriptor in x0
    - On failure, returns -1
- close:
    - x0 = file descriptor
    - x8 = 57
    - On success, returns 0
    - On failure, returns -1
- lseek:
    - x0 = file descriptor
    - x1 = offset (signed 64-bit)
    - x2 = whence (0 = SEEK_SET, 1 = SEEK_CUR, 2 = SEEK_END)
    - x8 = 62
    - On success, returns new offset in x0
    - On failure, returns -1
- read:
    - x0 = file descriptor
    - x1 = pointer to buffer
    - x2 = number of bytes to read
    - x8 = 63
    - On success, reads up to x2 bytes into buffer, returns number of bytes read in x0
    - On failure, returns -1
- write:
    - x0 = file descriptor
    - x1 = pointer to buffer
    - x2 = number of bytes to write
    - x8 = 64
    - If x0 is 1 or 2, writes to stdout or stderr respectively, returns number of bytes written in x0
    - For other file descriptors:
        - If file descriptor points to a file that is not read only:
            - Writes up to x2 bytes from buffer to file, returns number of bytes written in x0
        - If file descriptor points to a read-only file or unsupported device:
            - Returns 0

## Process Management
- exit:
    - x0 = exit code
    - x8 = 93
    - Does not return, halts the emulator
