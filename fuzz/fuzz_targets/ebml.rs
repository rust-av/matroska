#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate matroska;

use matroska::ebml::parse_element;

fuzz_target!(|data: &[u8]| {
    let _ = parse_element(data);
});
