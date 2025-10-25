use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

// use crate::types::{EmuError, EmuResult};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAccessMode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
    Append,
}

#[derive(Debug, Clone)]
pub struct VirtualFileHandle {
    pub host_path: PathBuf,
    pub position: usize,
    pub mode: FileAccessMode,
}

#[derive(Debug, Clone)]
pub struct VirtualFileSystem {
    pub root_path: PathBuf,
    pub fd_map: HashMap<u64, VirtualFileHandle>,
    pub next_fd: u64,
}

impl VirtualFileSystem {
    pub fn new(root: &str) -> Self {
        Self {
            root_path: PathBuf::from(root),
            fd_map: HashMap::new(),
            next_fd: 3,
        }
    }

    pub fn open(&mut self, rel_path: &str, flags: u64) -> std::io::Result<u64> {
        let rel_path_clean = rel_path.trim_start_matches('/');
        let full_path = self.root_path.join(rel_path_clean);

        if !full_path.starts_with(&self.root_path) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "unsafe path",
            ));
        }

        // Permission bits: O_RDONLY(0), O_WRONLY(1), O_RDWR(2)
        let access_mode = match flags & 0o3 {
            0 => FileAccessMode::ReadOnly,
            1 => FileAccessMode::WriteOnly,
            2 => FileAccessMode::ReadWrite,
            _ => FileAccessMode::ReadOnly,
        };

        let create = flags & 0o100 != 0; // O_CREAT
        let truncate = flags & 0o1000 != 0; // O_TRUNC
        let append = flags & 0o2000 != 0; // O_APPEND

        let mut opts = OpenOptions::new();
        opts.read(matches!(
            access_mode,
            FileAccessMode::ReadOnly | FileAccessMode::ReadWrite
        ));
        opts.write(matches!(
            access_mode,
            FileAccessMode::WriteOnly | FileAccessMode::ReadWrite
        ));
        opts.append(append);
        opts.create(create);
        opts.truncate(truncate);

        opts.open(&full_path)?; // Validate accessibility

        let fd = self.next_fd;
        self.next_fd += 1;

        self.fd_map.insert(
            fd,
            VirtualFileHandle {
                host_path: full_path,
                position: 0,
                mode: access_mode,
            },
        );

        Ok(fd)
    }

    pub fn read(&mut self, fd: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        if let Some(fh) = self.fd_map.get_mut(&fd) {
            let mut file = File::open(&fh.host_path)?;
            file.seek(SeekFrom::Start(fh.position as u64))?;
            let bytes_read = file.read(buf)?;
            fh.position += bytes_read;
            Ok(bytes_read)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "FD not found",
            ))
        }
    }

    pub fn write(&mut self, fd: u64, data: &[u8]) -> std::io::Result<usize> {
        if let Some(fh) = self.fd_map.get_mut(&fd) {
            match fh.mode {
                FileAccessMode::ReadOnly => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "read-only file",
                    ));
                }
                _ => (),
            }

            let mut opts = OpenOptions::new();
            opts.write(true).append(fh.mode == FileAccessMode::Append);
            let mut file = opts.open(&fh.host_path)?;

            if fh.mode != FileAccessMode::Append {
                file.seek(SeekFrom::Start(fh.position as u64))?;
            }

            file.write_all(data)?;
            fh.position += data.len();
            Ok(data.len())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "FD not found",
            ))
        }
    }

    pub fn lseek(&mut self, fd: u64, offset: i64, whence: u32) -> std::io::Result<u64> {
        if let Some(fh) = self.fd_map.get_mut(&fd) {
            use std::cmp::max;
            let file_len = std::fs::metadata(&fh.host_path)?.len() as i64;
            let new_pos = match whence {
                0 => offset,                      // SEEK_SET
                1 => fh.position as i64 + offset, // SEEK_CUR
                2 => file_len + offset,           // SEEK_END
                _ => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "invalid whence",
                    ));
                }
            };
            fh.position = max(0, new_pos) as usize;
            Ok(fh.position as u64)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "FD not found",
            ))
        }
    }

    pub fn close(&mut self, fd: u64) -> std::io::Result<()> {
        self.fd_map
            .remove(&fd)
            .map(|_| ())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "FD not found"))
    }
}
