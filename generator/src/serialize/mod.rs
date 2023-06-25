use std::ops::Deref;
use crate::serialize::decode::{Decode, decode_vec, Decoded};
use crate::serialize::encode::{Encode, encode_vec};
use crate::serialize::offset_ref::OffsetRef;

mod offset_ref;
mod encode;
mod decode;

fn round_trip<T>(x: &T) -> Decoded<T::Output> where T: Encode, T::Output: Decode {
    let data = encode_vec(x);
    decode_vec(data).unwrap()
}

pub unsafe trait Flat: 'static + Copy + Encode<Output=Self> + Decode {}

macro_rules! derive_flat {
    ($T : ident) => {
        unsafe impl $crate::serialize::Flat for $T{}
        impl $crate::serialize::Encode for $T {
            type Output = $T;
            fn encode(&self, position: usize, encoder: &mut $crate::serialize::encode::Encoder) { encoder.write_at(position, self) }
        }
        unsafe impl $crate::serialize::Decode for $T {
            unsafe fn decode(self: *const Self, bounds: &[u8]) -> Result<(), $crate::serialize::decode::DecodeError> { Ok(()) }
        }
    }
}
//
// macro_rules! derive_non_flat {
//     ($T: ident {$($x:ident),*} ) =>{
//         impl $crate::serialize::Encode for $T {
//             type Output = $T;
//             fn encode(&self, position: usize, encoder: &mut $crate::serialize::encode::Encoder) {
//                 $(
//                 self.$x.encode(position + offset_of!(Self::Output,$x), encoder);
//                 )*
//             }
//         }
//         unsafe impl $crate::serialize::Decode for $T {
//             unsafe fn decode(self: *const Self, bounds: &[u8]) -> Result<(), $crate::serialize::decode::DecodeError> { Ok(()) }
//         }
//     }
// }

derive_flat!(usize);

#[test]
fn test_usize() {
    assert_eq!(*round_trip(&123usize), 123usize);
}

#[test]
fn test_pair() {
    assert_eq!(*round_trip(&(1usize, 2usize)), (1usize, 2usize));
}

#[test]
fn test_ptr() {
    assert_eq!(**round_trip(&(Box::new(123))), 123);
}

#[test]
fn test_ptr2() {
    let output = round_trip(&(Box::new(Box::new(123))));
    assert_eq!(***output, 123);
}