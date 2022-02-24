#![recursion_limit = "256"]
#![allow(clippy::unreadable_literal)]
// TODO: avoid these or replace
//#![allow(clippy::large_enum_variant)]
//
#[macro_use]
extern crate cookie_factory;

extern crate av_data;
extern crate av_format;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

// use av_data::rational;

pub mod ebml;
pub mod elements;
#[macro_use]
pub mod serializer;
pub mod demuxer;
pub mod muxer;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
