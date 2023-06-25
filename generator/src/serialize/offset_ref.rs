use std::fmt::{Debug, Formatter};
use std::fs::Metadata;
use std::marker::PhantomData;
use std::mem;
use std::mem::{size_of, size_of_val_raw};
use std::ops::{Deref, DerefMut};
use std::ptr::{from_raw_parts, Pointee};
use crate::serialize::decode::{Decode, DecodeError};
use crate::serialize::encode::{Encode, Encoder};

#[repr(C)]
pub struct OffsetRef<T: ?Sized> {
    offset: usize,
    metadata: <T as Pointee>::Metadata,
    phantom: PhantomData<T>,
}

impl<T: ?Sized> OffsetRef<T> {
    pub fn get_raw(&self) -> *const T {
        unsafe {
            from_raw_parts((self as *const Self as *const u8).add(self.offset) as *const (), self.metadata)
        }
    }
}

impl<T: ?Sized> Deref for OffsetRef<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.get_raw() }
    }
}

impl<T: ?Sized + Encode> Encode for Box<T> {
    type Output = OffsetRef<T::Output>;
    fn encode(&self, position: usize, encoder: &mut Encoder) {
        let inner_position = encoder.write(&**self);
        encoder.write_at::<usize>(position, &(inner_position - position));
    }
}

unsafe impl<T: ?Sized + Decode> Decode for OffsetRef<T> {
    unsafe fn decode(self: *const Self, bounds: &[u8]) -> Result<(), DecodeError> {
        unsafe {
            let remainder = bounds.as_ptr_range().end.offset_from(self as *const u8) as usize;
            if (*self).offset > remainder {
                return Err(DecodeError);
            }
            let inner = (*self).get_raw();
            let extra = size_of_val_raw(inner);
            let remainder2 = bounds.as_ptr_range().end.offset_from(inner as *const u8) as usize;
            if extra > remainder2 {
                return Err(DecodeError);
            }
            inner.decode(bounds)?;
            Ok(())
        }
    }
}