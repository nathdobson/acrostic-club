use std::alloc::Layout;
use std::mem::{align_of, offset_of, size_of};
use std::slice;
use crate::serialize::Flat;

pub fn encode_vec<T: Encode>(x: &T) -> Vec<u8> {
    let mut encoder = Encoder { buffer: vec![] };
    encoder.write::<T>(x);
    encoder.buffer
}

pub struct Encoder {
    buffer: Vec<u8>,
}

pub trait Encode {
    type Output;
    fn encode(&self, position: usize, encoder: &mut Encoder);
}

impl Encoder {
    pub fn new() -> Self { Encoder { buffer: vec![] } }
    pub fn write_at<T: Flat>(&mut self, position: usize, object: &T) {
        unsafe {
            let value = slice::from_raw_parts(object as *const T as *const u8, size_of::<T>());
            self.buffer[position..position + size_of::<T>()].copy_from_slice(value);
        }
    }
    pub fn write<T: ?Sized + Encode>(&mut self, object: &T) -> usize {
        let align = Layout::from_size_align(self.buffer.len(), align_of::<T::Output>()).unwrap().pad_to_align();
        self.buffer.resize(align.size(), 0);
        let position = self.buffer.len();
        self.buffer.resize(self.buffer.len() + size_of::<T::Output>(), 0);
        object.encode(position, self);
        position
    }
}


impl<A: Encode, B: Encode> Encode for (A, B) {
    type Output = (A::Output, B::Output);
    fn encode(&self, position: usize, encoder: &mut Encoder) {
        self.0.encode(position + offset_of!(Self::Output,0), encoder);
        self.1.encode(position + offset_of!(Self::Output,1), encoder);
    }
}
