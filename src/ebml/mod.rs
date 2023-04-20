mod error;
mod macros;
mod parse;

#[cfg(test)]
mod tests;

pub use crate::impl_ebml_master;
use crate::permutation::matroska_permutation;

pub use self::error::{ebml_err, Error, ErrorKind};
pub use self::parse::*;

impl_ebml_master! {
    // Element ID 0x1A45DFA3
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct EbmlHeader {
        [0x4286] version: (u32) = 1,
        [0x42F7] read_version: (u32) = 1,
        [0x42F2] max_id_length: (u32) = 4,
        [0x42F3] max_size_length: (u32) = 8,
        [0x4282] doc_type: (String),
        [0x4287] doc_type_version: (u32) = 1,
        [0x4285] doc_type_read_version: (u32) = 1,
    }
}

pub fn ebml_header(input: &[u8]) -> EbmlResult<EbmlHeader> {
    ebml_element(0x1A45DFA3)(input)
}
