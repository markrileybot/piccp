use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::BufMut;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub const FRAME_TYPE_CTS: u8 = 0x01;
pub const FRAME_TYPE_DONE: u8 = 0x02;
pub const FRAME_TYPE_DATA: u8 = 0x03;

///
/// The thing that's exchanged
///
#[derive(Debug, Clone)]
pub struct Frame {
    encoded: Vec<u8>,
}

impl Frame {
    pub fn new(encoded: Vec<u8>) -> Self {
        return Self {
            encoded
        }
    }

    pub fn new_cts(offset: usize) -> Self {
        let mut encoded: Vec<u8> = Vec::with_capacity(4 + 1 + 4);
        encoded.put_u32(COUNTER.fetch_add(1, Ordering::AcqRel) as u32);
        encoded.put_u8(FRAME_TYPE_CTS);
        encoded.put_u32(offset as u32);
        return Self {
            encoded
        }
    }

    pub fn new_done() -> Self {
        let mut encoded: Vec<u8> = Vec::with_capacity(4 + 1);
        encoded.put_u32(COUNTER.fetch_add(1, Ordering::AcqRel) as u32);
        encoded.put_u8(FRAME_TYPE_DONE);
        return Self {
            encoded
        }
    }

    pub fn new_data<D>(offset: usize, data: D) -> Self
        where D: AsRef<[u8]> {
        let d = data.as_ref();
        let mut encoded: Vec<u8> = Vec::with_capacity(4 + 1 + 4 + d.len());
        encoded.put_u32(COUNTER.fetch_add(1, Ordering::AcqRel) as u32);
        encoded.put_u8(FRAME_TYPE_DATA);
        encoded.put_u32(offset as u32);
        encoded.put_slice(d);
        return Self {
            encoded
        };
    }

    pub fn get_sequence(&self) -> usize {
        return u32::from_be_bytes(self.encoded[0..4].try_into().unwrap()) as usize;
    }

    pub fn get_type(&self) -> u8 {
        return self.encoded[4];
    }

    pub fn get_offset(&self) -> usize {
        return u32::from_be_bytes(self.encoded[5..9].try_into().unwrap()) as usize;
    }

    pub fn get_data(&self) -> &[u8] {
        return &self.encoded[9..];
    }

    pub fn is_done(&self) -> bool {
        return self.get_type() == FRAME_TYPE_DONE;
    }
}

impl AsRef<[u8]> for Frame {
    fn as_ref(&self) -> &[u8] {
        return &self.encoded;
    }
}