#![feature(trace_macros)]
#![recursion_limit = "256"]
#[macro_use]
extern crate nom;
#[macro_use]
extern crate cookie_factory;

#[macro_use]
extern crate log;

extern crate av_data;
extern crate av_format;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

use av_data::rational;

#[macro_use]
pub mod permutation;
#[macro_use]
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
