use base64::{encode_config_buf, URL_SAFE_NO_PAD as ENCODE_PRESET};
use blake3::Hasher as State;
use std::hash::{Hash, Hasher};

pub struct DataHasher {
    state: State,
}

const CONTEXT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " ",
    env!("CARGO_PKG_VERSION"),
    " ",
    env!("CARGO_CRATE_NAME")
);

impl Default for DataHasher {
    fn default() -> Self {
        let state = State::new_derive_key(CONTEXT);
        Self { state }
    }
}

impl Hasher for DataHasher {
    fn write(&mut self, bytes: &[u8]) {
        self.state.update(bytes);
    }

    fn finish(&self) -> u64 {
        unimplemented!();
    }
}

impl DataHasher {
    pub fn hash<T: Hash>(&mut self, val: &T) {
        val.hash(self);
    }

    pub fn finish_binary_vec(&self) -> Vec<u8> {
        let mut out = Default::default();
        self.finish_binary_to_vec(&mut out);
        out
    }

    pub fn finish_binary_to_vec(&self, out: &mut Vec<u8>) {
        out.extend(self.state.finalize().as_bytes());
    }

    pub fn finish_base64_string(&self) -> String {
        let mut out = Default::default();
        self.finish_base64_to_string(&mut out);
        out
    }

    pub fn finish_base64_to_string(&self, out: &mut String) {
        encode_config_buf(self.state.finalize().as_bytes(), ENCODE_PRESET, out);
    }

    pub fn hash_binary_vec<T: Hash>(val: &T) -> Vec<u8> {
        let mut out = Default::default();
        Self::hash_binary_to_vec(val, &mut out);
        out
    }

    pub fn hash_binary_to_vec<T: Hash>(val: &T, out: &mut Vec<u8>) {
        let mut this = Self::default();
        this.hash(val);
        this.finish_binary_to_vec(out);
    }

    pub fn hash_base64_string<T: Hash>(val: &T) -> String {
        let mut out = Default::default();
        Self::hash_base64_to_string(val, &mut out);
        out
    }

    pub fn hash_base64_to_string<T: Hash>(val: &T, out: &mut String) {
        let mut this = Self::default();
        this.hash(val);
        this.finish_base64_to_string(out);
    }
}
