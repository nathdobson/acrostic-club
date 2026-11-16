use std::alloc::{AllocError, Allocator, Layout};
use std::mem::size_of;
use std::path::Path;
use std::ptr::{NonNull, slice_from_raw_parts};
use std::{io, slice};

use memmap::{MmapMut, MmapOptions};
use tokio::fs::File;

use crate::write_path;

pub struct MmapAllocator {
    file: File,
    mmap: Option<MmapMut>,
}

unsafe impl Allocator for MmapAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        todo!()
    }
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {}
}

// pub unsafe trait AnyRepr: 'static + Send + Sync {}
//
// unsafe impl AnyRepr for u32 {}
//
// unsafe impl AnyRepr for LetterSet {}
//
// unsafe impl<A, B> AnyRepr for (A, B)
//     where
//         A: AnyRepr,
//         B: AnyRepr,
// {}

pub async fn save_vec<T>(file: &Path, value: &[T]) -> io::Result<()> {
    unsafe {
        let start = value.as_ptr();
        let end = start.offset(value.len() as isize);
        let start = start as *const u8;
        let end = end as *const u8;
        let slice = &*slice_from_raw_parts(start, end.offset_from(start) as usize);
        write_path(file, slice).await?;
        Ok(())
    }
}

pub async fn restore_vec<T>(filename: &Path) -> io::Result<Box<[T], MmapAllocator>> {
    Ok(unsafe {
        let file = tokio::fs::OpenOptions::new()
            .read(true)
            .open(filename)
            .await
            .unwrap_or_else(|e| panic!("Cannot open {:?}: {}", filename, e));
        let len = file.metadata().await?.len();
        if len == 0 {
            Box::from_raw_in(&mut [], MmapAllocator { mmap: None, file })
        } else {
            let file = file.into_std().await;
            let mmap = MmapOptions::new().map_copy(&file).unwrap();
            let file = tokio::fs::File::from_std(file);
            assert_eq!(mmap.len() as u64, len);
            assert_eq!(mmap.len() % size_of::<T>(), 0);
            Box::from_raw_in(
                slice::from_raw_parts_mut(mmap.as_ptr() as *mut T, mmap.len() / size_of::<T>()),
                MmapAllocator {
                    mmap: Some(mmap),
                    file,
                },
            )
        }
    })
}

// pub struct LazyFile<T>(LazyAsync<io::Result<Box<[T], MmapAllocator>>>);

// impl<T: AnyRepr> LazyFile<T> {
//     pub const fn new(path: fn() -> PathBuf) -> Self {
//         LazyFile(LazyAsync::new())
//     }
//     pub async fn get() -> io::Result<&'static [Self]> {
//         Ok(&*(FLAT_WORDS.get_io().await?))
//     }
// }

// pub static FLAT_WORDS: LazyAsync<io::Result<Box<[FlatWord], MmapAllocator>>> = LazyAsync::new(|| async {
//     Ok(restore_vec(&PACKAGE_PATH.join("build/dict.dat")).await?)
// }.boxed());
