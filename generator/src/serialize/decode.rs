use core::slice;
use std::marker::PhantomData;
use std::mem::{offset_of, size_of};
use std::ops::Deref;
use crate::serialize::Flat;

#[derive(Copy, Clone, Debug)]
pub struct DecodeError;

pub unsafe trait Decode {
    unsafe fn decode(self: *const Self, bounds: &[u8]) -> Result<(), DecodeError>;
}



unsafe impl<A, B> Decode for (A, B) where A: Decode, B: Decode {
    unsafe fn decode(self: *const Self, bounds: &[u8]) -> Result<(), DecodeError> {
        (&raw const (*self).0).decode(bounds)?;
        (&raw const (*self).1).decode(bounds)?;
        Ok(())
    }
}

fn decode_slice<T: Decode>(slice: &[u8]) -> Result<&T, DecodeError> {
    unsafe {
        (slice as *const [u8] as *const u8 as *const T).decode(slice)?;
        Ok(&*(slice as *const [u8] as *const u8 as *const T))
    }
}

pub struct Decoded<T> {
    vec: Vec<u8>,
    phantom: PhantomData<T>,
}

impl<T> Deref for Decoded<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe {
            &*((&*self.vec) as *const [u8] as *const T)
        }
    }
}

pub fn decode_vec<T: Decode>(vec: Vec<u8>) -> Result<Decoded<T>, DecodeError> {
    decode_slice::<T>(&vec)?;
    Ok(Decoded { vec, phantom: PhantomData })
}