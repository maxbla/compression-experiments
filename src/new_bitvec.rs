use std::convert::From;
use std::convert::Into;
use std::hash::Hash;
use std::hash::Hasher;

use crate::BitVec;

/// A newtype wrapper around bitvec that provides a faster hashing implementation
#[derive(Eq, PartialEq, Clone)]
pub struct NewBitVec {
    inner: BitVec
}

impl NewBitVec {
    pub fn new() -> NewBitVec {
        NewBitVec{inner: BitVec::new()}
    }

    pub fn push(&mut self, bit:bool) {
        self.inner.push(bit);
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl From<BitVec> for NewBitVec {
    fn from(bv: BitVec) -> NewBitVec {
        NewBitVec{inner: bv}
    }
}

impl Into<BitVec> for NewBitVec {
    fn into(self) -> BitVec {
        self.inner
    }
}

impl Hash for NewBitVec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.inner.as_slice());
        state.write_usize(self.inner.len());
    }
}