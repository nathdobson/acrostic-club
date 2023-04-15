use std::alloc::{AllocError, Allocator, Layout};
use std::fmt::Debug;
use std::fs::File;
use std::mem::size_of;
use std::path::Path;
use std::ptr::{null_mut, slice_from_raw_parts, NonNull};
use std::{fs, slice};

use memmap::{Mmap, MmapMut, MmapOptions};

pub struct MmapAllocator {
    file: File,
    mmap: Option<MmapMut>,
}

unsafe impl Allocator for MmapAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> { todo!() }
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {}
}

pub unsafe trait AnyRepr {}

unsafe impl AnyRepr for u32 {}

unsafe impl<A, B> AnyRepr for (A, B)
where
    A: AnyRepr,
    B: AnyRepr,
{
}

pub fn save_vec<T: AnyRepr>(file: &Path, value: &[T]) {
    unsafe {
        let start = value.as_ptr();
        let end = start.offset(value.len() as isize);
        let start = start as *const u8;
        let end = end as *const u8;
        let slice = &*slice_from_raw_parts(start, end.offset_from(start) as usize);
        fs::write(file, slice).unwrap();
    }
}

pub fn restore_vec<T: AnyRepr>(filename: &Path) -> Box<[T], MmapAllocator> {
    unsafe {
        let file = fs::OpenOptions::new()
            .read(true)
            .open(filename)
            .unwrap_or_else(|e| panic!("Cannot open {:?}: {}", filename, e));
        let len = file.metadata().unwrap().len();
        if len == 0 {
            Box::from_raw_in(&mut [], MmapAllocator { mmap: None, file })
        } else {
            let mmap = MmapOptions::new().map_copy(&file).unwrap();
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
    }
}

#[test]
fn test_mmap_allocator() {
    let file = "/tmp/test_mmap_allocator";
    let value = vec![1u32, 2, 3].into_boxed_slice();
    save_vec(Path::new(file), &value);
    assert_eq!(*value, *restore_vec::<u32>(Path::new(file)));
}
