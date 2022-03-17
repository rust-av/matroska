#![no_main]

use libfuzzer_sys::fuzz_target;
use matroska::ebml::ebml_header;

fuzz_target!(|data: &[u8]| {
    let _ = ebml_header(data);
});
