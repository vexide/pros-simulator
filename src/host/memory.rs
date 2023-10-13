use std::sync::atomic::{AtomicU8, Ordering};

use wasmtime::{MemoryAccessError, SharedMemory};

#[derive(Debug)]
pub struct OutOfBoundsError;

pub trait SharedMemoryExt {
    fn read_c_str(&self, ptr: u32) -> anyhow::Result<String>;
    fn write_relaxed(&self, offset: usize, buffer: &[u8]) -> Result<(), OutOfBoundsError>;
    fn read_relaxed(&self, offset: usize, length: usize) -> Result<Vec<u8>, OutOfBoundsError>;
}

impl SharedMemoryExt for SharedMemory {
    fn read_c_str(&self, ptr: u32) -> anyhow::Result<String> {
        let data = self.data().get(ptr as usize..).unwrap();
        for (index, cell) in data.iter().enumerate() {
            if unsafe { cell.get().read() } == 0 {
                return Ok(String::from_utf8(
                    data[..index]
                        .iter()
                        .map(|c| unsafe { c.get().read() })
                        .collect::<Vec<_>>(),
                )
                .expect("invalid UTF-8 string"));
            }
        }

        Err(anyhow::anyhow!("C string must be null-terminated"))
    }
    fn write_relaxed(&self, offset: usize, buffer: &[u8]) -> Result<(), OutOfBoundsError> {
        let Some(data) = self.data().get(offset..offset + buffer.len()) else {
            return Err(OutOfBoundsError);
        };
        for (cell, byte) in data.iter().zip(buffer) {
            unsafe { cell.get().write(*byte) };
        }
        Ok(())
    }
    fn read_relaxed(&self, offset: usize, length: usize) -> Result<Vec<u8>, OutOfBoundsError> {
        let Some(data) = self.data().get(offset..offset + length) else {
            return Err(OutOfBoundsError);
        };
        let mut buffer = Vec::with_capacity(length);
        for cell in data.iter() {
            buffer.push(unsafe { cell.get().read() });
        }
        Ok(buffer)
    }
}
