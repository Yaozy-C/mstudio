//! Three bounded BGRA slots shared by the worker and the app. A reader pins a
//! slot before touching pixels; a producer can only replace an unpinned slot.
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::PathBuf,
    ptr::NonNull,
    sync::{
        Mutex,
        atomic::{AtomicU32, Ordering},
    },
};
const FREE: u32 = 0;
const WRITING: u32 = 1;
const READABLE: u32 = 2;
const READING: u32 = 3;
const MAGIC: u32 = 0x4d535431;
#[repr(C, align(64))]
struct Header {
    magic: u32,
    width: u32,
    height: u32,
    latest: AtomicU32,
    slots: [AtomicU32; 3],
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Descriptor {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
}
pub struct SharedFrames {
    descriptor: Descriptor,
    base: NonNull<u8>,
    length: usize,
    stride: usize,
    owner: bool,
    access: Mutex<()>,
}
// The mapping lives as long as self. Header fields are immutable or atomic;
// slot ownership uses acquire/release CAS across processes. Local access is serial.
unsafe impl Send for SharedFrames {}
unsafe impl Sync for SharedFrames {}
fn layout(width: u32, height: u32) -> Result<(usize, usize), String> {
    if width == 0 || height == 0 || width > 3840 || height > 3840 {
        return Err("预览共享帧尺寸无效".into());
    }
    let stride = (16 + width as usize * height as usize * 4 + 63) & !63;
    Ok((stride, size_of::<Header>() + stride * 3))
}
impl SharedFrames {
    pub fn create(width: u32, height: u32) -> Result<Self, String> {
        let (_, length) = layout(width, height)?;
        let descriptor = Descriptor {
            path: std::env::temp_dir()
                .join(format!("mstudio-preview-{}.frames", uuid::Uuid::new_v4())),
            width,
            height,
        };
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&descriptor.path)
            .map_err(|e| e.to_string())?;
        let result = (|| {
            file.set_len(length as u64).map_err(|e| e.to_string())?;
            let mapped = Self::map(&file, descriptor.clone(), true)?;
            // The descriptor is not published to the worker until initialization finishes.
            unsafe {
                mapped.base.as_ptr().cast::<Header>().write(Header {
                    magic: MAGIC,
                    width,
                    height,
                    latest: AtomicU32::new(3),
                    slots: std::array::from_fn(|_| AtomicU32::new(FREE)),
                });
            }
            Ok(mapped)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&descriptor.path);
        }
        result
    }
    pub fn open(descriptor: Descriptor) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&descriptor.path)
            .map_err(|e| e.to_string())?;
        let mapped = Self::map(&file, descriptor, false)?;
        let h = mapped.header();
        if h.magic != MAGIC
            || h.width != mapped.descriptor.width
            || h.height != mapped.descriptor.height
        {
            return Err("预览共享帧头无效".into());
        }
        Ok(mapped)
    }
    fn map(file: &File, descriptor: Descriptor, owner: bool) -> Result<Self, String> {
        let (stride, length) = layout(descriptor.width, descriptor.height)?;
        if file.metadata().map_err(|e| e.to_string())?.len() != length as u64 {
            return Err("预览共享帧长度无效".into());
        }
        let address = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                length,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if address == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(Self {
            descriptor,
            base: NonNull::new(address.cast()).unwrap(),
            length,
            stride,
            owner,
            access: Mutex::new(()),
        })
    }
    pub fn descriptor(&self) -> Descriptor {
        self.descriptor.clone()
    }
    pub fn unlink(&self) {
        if self.owner {
            let _ = std::fs::remove_file(&self.descriptor.path);
        }
    }
    fn header(&self) -> &Header {
        unsafe { &*self.base.as_ptr().cast::<Header>() }
    }
    fn slot(&self, index: usize) -> *mut u8 {
        unsafe {
            self.base
                .as_ptr()
                .add(size_of::<Header>() + index * self.stride)
        }
    }
    pub fn publish(
        &self,
        seq: u32,
        position: u32,
        data: &[u8],
        row_stride: usize,
    ) -> Result<bool, String> {
        let width = self.descriptor.width as usize * 4;
        let height = self.descriptor.height as usize;
        if row_stride < width || data.len() < (height - 1) * row_stride + width {
            return Err("预览共享帧像素不完整".into());
        }
        let _access = self.access.lock().unwrap();
        let h = self.header();
        let latest = h.latest.load(Ordering::Acquire) as usize;
        // Prefer a different slot, keeping the latest frame available during upload.
        for offset in 1..=3 {
            let index = (latest.min(2) + offset) % 3;
            let state = h.slots[index].load(Ordering::Acquire);
            if !matches!(state, FREE | READABLE)
                || h.slots[index]
                    .compare_exchange(state, WRITING, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
            {
                continue;
            }
            let ptr = self.slot(index);
            unsafe {
                for (i, value) in [seq, self.descriptor.width, self.descriptor.height, position]
                    .iter()
                    .enumerate()
                {
                    std::ptr::copy_nonoverlapping(value.to_le_bytes().as_ptr(), ptr.add(i * 4), 4);
                }
                for row in 0..height {
                    std::ptr::copy_nonoverlapping(
                        data.as_ptr().add(row * row_stride),
                        ptr.add(16 + row * width),
                        width,
                    );
                }
            }
            h.slots[index].store(READABLE, Ordering::Release);
            h.latest.store(index as u32, Ordering::Release);
            return Ok(true);
        }
        Ok(false)
    }
    pub fn with_frame<T>(&self, expected: Option<u32>, read: impl FnOnce(&[u8]) -> T) -> Option<T> {
        let _access = self.access.lock().unwrap();
        let h = self.header();
        let index = h.latest.load(Ordering::Acquire) as usize;
        if index >= 3
            || h.slots[index]
                .compare_exchange(READABLE, READING, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return None;
        }
        struct Unpin<'a>(&'a AtomicU32);
        impl Drop for Unpin<'_> {
            fn drop(&mut self) {
                self.0.store(READABLE, Ordering::Release);
            }
        }
        let _pin = Unpin(&h.slots[index]);
        let bytes = unsafe {
            std::slice::from_raw_parts(
                self.slot(index),
                16 + self.descriptor.width as usize * self.descriptor.height as usize * 4,
            )
        };
        if expected.is_some_and(|seq| u32::from_le_bytes(bytes[..4].try_into().unwrap()) != seq) {
            return None;
        }
        Some(read(bytes))
    }
}
#[cfg(test)]
#[path = "preview_shared_tests.rs"]
mod tests;
impl Drop for SharedFrames {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.base.as_ptr().cast(), self.length);
        }
        self.unlink();
    }
}
