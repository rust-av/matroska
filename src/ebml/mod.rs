mod error;
mod parse;

#[cfg(test)]
mod tests;

use nom::combinator::opt;

use crate::permutation::matroska_permutation;

pub use self::error::{ebml_err, Error, ErrorKind};
pub use self::parse::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbmlHeader {
    pub version: u32,
    pub read_version: u32,
    pub max_id_length: u32,
    pub max_size_length: u32,
    pub doc_type: String,
    pub doc_type_version: u32,
    pub doc_type_read_version: u32,
}

pub fn ebml_header(input: &[u8]) -> EbmlResult<EbmlHeader> {
    master(0x1A45DFA3, |i| {
        matroska_permutation((
            opt(u32(0x4286)), // version
            opt(u32(0x42F7)), // read_version
            opt(u32(0x42F2)), // max id length
            opt(u32(0x42F3)), // max size length
            str(0x4282),      // doctype
            opt(u32(0x4287)), // doctype version
            opt(u32(0x4285)), // doctype_read version
        ))(i)
        .map(|(i, t)| {
            (
                i,
                EbmlHeader {
                    version: t.0.unwrap_or(1),
                    read_version: t.1.unwrap_or(1),
                    max_id_length: t.2.unwrap_or(4),
                    max_size_length: t.3.unwrap_or(8),
                    doc_type: t.4,
                    doc_type_version: t.5.unwrap_or(1),
                    doc_type_read_version: t.6.unwrap_or(1),
                },
            )
        })
    })(input)
}
