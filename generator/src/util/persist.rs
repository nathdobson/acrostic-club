use std::alloc::{Allocator, AllocError, Layout};
use std::{io, slice};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use memmap::{MmapMut, MmapOptions};
use rkyv::{Archive, Archived, check_archived_root, CheckBytes, Serialize};
use rkyv::ser::serializers::AllocSerializer;
use rkyv::validation::CheckTypeError;
use rkyv::validation::validators::DefaultValidator;
use safe_once_async::sync::{AsyncLazyLock, AsyncOnceLock, AsyncStaticLock};
use tokio::fs::File;
use crate::{read_path, write_path};
use crate::util::lazy_async::CloneError;

pub struct MmapAllocator {
    file: File,
    mmap: Option<MmapMut>,
}

unsafe impl Allocator for MmapAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> { todo!() }
    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {}
}

pub async fn mmap_bytes(filename: &Path) -> io::Result<Box<[u8], MmapAllocator>> {
    Ok(unsafe {
        let file = tokio::fs::OpenOptions::new()
            .read(true)
            .open(filename).await
            .unwrap_or_else(|e| panic!("Cannot open {:?}: {}", filename, e));
        let len = file.metadata().await?.len();
        if len == 0 {
            Box::from_raw_in(&mut [], MmapAllocator { mmap: None, file })
        } else {
            let file = file.into_std().await;
            let mmap = MmapOptions::new().map_copy(&file).unwrap();
            let file = tokio::fs::File::from_std(file);
            assert_eq!(mmap.len() as u64, len);
            Box::from_raw_in(
                slice::from_raw_parts_mut(mmap.as_ptr() as *mut u8, mmap.len()),
                MmapAllocator {
                    mmap: Some(mmap),
                    file,
                },
            )
        }
    })
}

pub struct ArchivedOwned<T: Archive, A: Allocator> {
    b: Box<[u8], A>,
    r: *const Archived<T>,
}

impl<T: Archive, A: Allocator> ArchivedOwned<T, A> {
    pub fn new(b: Box<[u8], A>) -> anyhow::Result<Self> where for<'a> T::Archived: CheckBytes<DefaultValidator<'a>> {
        let r: *const _ = check_archived_root::<T>(&*b).map_err(|e| anyhow::Error::msg(format!("{:?}", e)))?;
        Ok(ArchivedOwned { b, r })
    }
}

impl<T: Archive, A: Allocator> Deref for ArchivedOwned<T, A> {
    type Target = T::Archived;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.r }
    }
}

pub struct PersistentFile<T: Archive> {
    path: PathBuf,
    value: AsyncLazyLock<anyhow::Result<ArchivedOwned<T, MmapAllocator>>>,
}

unsafe impl<T: Archive, A: Allocator + Send> Send for ArchivedOwned<T, A> where Archived<T>: Send {}

unsafe impl<T: Archive, A: Allocator + Sync> Sync for ArchivedOwned<T, A> where Archived<T>: Sync {}

impl<T: 'static + Archive> PersistentFile<T> where
        for<'a> T::Archived: Send + CheckBytes<DefaultValidator<'a>>,
        T: Serialize<AllocSerializer<256>> {
    pub fn new(path: &Path) -> Self {
        PersistentFile {
            path: path.to_path_buf(),
            value: AsyncLazyLock::new({
                let path = path.to_path_buf();
                async move {
                    Ok(ArchivedOwned::new(mmap_bytes(&path).await?)?)
                }
            }),
        }
    }
    pub async fn get_static(&'static self) -> anyhow::Result<&'static T::Archived> {
        Ok(&*self.value.get().await.clone_error_static()?)
    }
    pub async fn set(&self, value: &T) -> io::Result<()> {
        write_path(&self.path, &rkyv::to_bytes::<_, 256>(value).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidInput, e)
        })?).await?;
        Ok(())
    }
}

// pub async fn save_rkyv<T: Archive + Serialize<AllocSerializer<256>>>(file: &Path, value: &T) -> io::Result<()> {
//     write_path(file, &rkyv::to_bytes::<_, 256>(value).map_err(|e| {
//         io::Error::new(io::ErrorKind::InvalidInput, e)
//     })?).await?;
//     Ok(())
// }
//
// pub async fn restore_rkyv<T: Archive>(file: &Path) -> io::Result<&'static Archived<T>> where for<'a> Archived<T>: CheckBytes<DefaultValidator<'a>> {
//     let data: &'static Vec<u8> = Box::leak(Box::new(read_path(file).await?));
//     Ok(rkyv::validation::validators::check_archived_root::<T>(&data).map_err(|e| { io::Error::new(io::ErrorKind::InvalidInput, format!("{:?}", e)) })?)
// }