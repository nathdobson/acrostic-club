use crate::util::lazy_async::CloneError;
use crate::{error_with_path, write_path};
use anyhow::Context;
use memmap::{MmapMut, MmapOptions};
use rkyv::ser::serializers::AllocSerializer;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, Archived, CheckBytes, Serialize, check_archived_root};
use safe_once_async::detached::{JoinTransparent, spawn_transparent};
use safe_once_async::sync::AsyncLazyLock;
use std::alloc::{AllocError, Allocator, Layout};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::{io, slice};

pub struct MmapAllocator {
    // Keeps alive the mapping that the box's slice points into.
    _mmap: Option<MmapMut>,
}

unsafe impl Allocator for MmapAllocator {
    fn allocate(&self, _layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        todo!()
    }
    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {}
}

pub async fn mmap_bytes(filename: &Path) -> io::Result<Box<[u8], MmapAllocator>> {
    Ok(unsafe {
        let file = tokio::fs::OpenOptions::new()
            .read(true)
            .open(filename)
            .await
            .unwrap_or_else(|e| panic!("Cannot open {filename:?}: {e}"));
        let len = file
            .metadata()
            .await
            .map_err(error_with_path("inspect", filename))?
            .len();
        if len == 0 {
            Box::from_raw_in(&mut [], MmapAllocator { _mmap: None })
        } else {
            let file = file.into_std().await;
            let mmap = MmapOptions::new()
                .map_copy(&file)
                .unwrap_or_else(|error| panic!("Cannot map {filename:?}: {error}"));
            assert_eq!(mmap.len() as u64, len);
            Box::from_raw_in(
                slice::from_raw_parts_mut(mmap.as_ptr() as *mut u8, mmap.len()),
                MmapAllocator { _mmap: Some(mmap) },
            )
        }
    })
}

pub struct ArchivedOwned<T: Archive, A: Allocator> {
    // Keeps alive the bytes that `r` points into.
    _bytes: Box<[u8], A>,
    r: *const Archived<T>,
}

impl<T: Archive, A: Allocator> ArchivedOwned<T, A> {
    pub fn new(b: Box<[u8], A>) -> anyhow::Result<Self>
    where
        for<'a> T::Archived: CheckBytes<DefaultValidator<'a>>,
    {
        let r: *const _ =
            check_archived_root::<T>(&*b).map_err(|e| anyhow::Error::msg(format!("{e:?}")))?;
        Ok(ArchivedOwned { _bytes: b, r })
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
    value: AsyncLazyLock<JoinTransparent<anyhow::Result<ArchivedOwned<T, MmapAllocator>>>>,
}

unsafe impl<T: Archive, A: Allocator + Send> Send for ArchivedOwned<T, A> where Archived<T>: Send {}

unsafe impl<T: Archive, A: Allocator + Sync> Sync for ArchivedOwned<T, A> where Archived<T>: Sync {}

impl<T: 'static + Archive> PersistentFile<T>
where
    for<'a> T::Archived: Send + CheckBytes<DefaultValidator<'a>>,
    T: Serialize<AllocSerializer<256>>,
{
    pub fn new(path: &Path) -> Self {
        PersistentFile {
            path: path.to_path_buf(),
            value: AsyncLazyLock::new({
                let path = path.to_path_buf();
                spawn_transparent(async move {
                    ArchivedOwned::new(mmap_bytes(&path).await?)
                        .with_context(|| format!("Cannot parse {path:?}"))
                })
            }),
        }
    }
    pub async fn get_static(&'static self) -> anyhow::Result<&'static T::Archived> {
        Ok(&*self.value.get().await.clone_error_static()?)
    }
    pub async fn set(&self, value: &T) -> io::Result<()> {
        write_path(
            &self.path,
            &rkyv::to_bytes::<_, 256>(value)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
        )
        .await?;
        Ok(())
    }
}
