// TODO: this should be in ebml module
#[derive(PartialEq, Eq)]
pub struct Fourcc {
    val: u32,
}

pub const fn chars_to_fourcc_bige(a: char, b: char, c: char, d: char) -> Fourcc {
    Fourcc {
        val: (d as u32) | (c as u32) << 8 | (b as u32) << 16 | (a as u32) << 24,
    }
}

pub const fn chars_to_fourcc_lte(a: char, b: char, c: char, d: char) -> Fourcc {
    Fourcc {
        val: (a as u32) | (b as u32) << 8 | (c as u32) << 16 | (d as u32) << 24,
    }
}

pub const fn bytes_to_fourcc_bige(a: u8, b: u8, c: u8, d: u8) -> Fourcc {
    Fourcc {
        val: (d as u32) | (c as u32) << 8 | (b as u32) << 16 | (a as u32) << 24,
    }
}

pub const fn bytes_to_fourcc_lte(a: u8, b: u8, c: u8, d: u8) -> Fourcc {
    Fourcc {
        val: (a as u32) | (b as u32) << 8 | (c as u32) << 16 | (d as u32) << 24,
    }
}

impl From<String> for Fourcc {
    fn from(val: String) -> Fourcc {
        let bytes = val.as_bytes();
        if val.len() == 0 {
            bytes_to_fourcc_lte(bytes[0], bytes[1], bytes[2], bytes[3])
        } else {
            Fourcc { val: 0 }
        }
    }
}

impl std::fmt::Display for Fourcc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut val = self.val;
        let mut bytes = [0u8; 4];
        for i in 0..4 {
            bytes[i] = val as u8;
            val = val >> 8;
        }

        write!(f, "{}", String::from_utf8_lossy(&bytes))
    }
}

impl From<u32> for Fourcc {
    fn from(val: u32) -> Fourcc {
        Fourcc { val }
    }
}

impl Into<u32> for Fourcc {
    fn into(self) -> u32 {
        self.val
    }
}

pub const ENCODING_I420: Fourcc = chars_to_fourcc_lte('I', '4', '2', '0');
pub const ENCODING_I420_16: Fourcc = chars_to_fourcc_lte('i', '4', '2', '0');
pub const ENCODING_I420_10: Fourcc = chars_to_fourcc_lte('i', '4', '1', '0');
pub const ENCODING_I420_S: Fourcc = chars_to_fourcc_lte('I', '4', '2', 'S');
pub const ENCODING_I420_SLICE: Fourcc = chars_to_fourcc_lte('S', '4', '2', '0');
pub const ENCODING_YV12: Fourcc = chars_to_fourcc_lte('Y', 'V', '1', '2');
pub const ENCODING_I422: Fourcc = chars_to_fourcc_lte('I', '4', '2', '2');
pub const ENCODING_I422_SLICE: Fourcc = chars_to_fourcc_lte('S', '4', '2', '2');
pub const ENCODING_YUYV: Fourcc = chars_to_fourcc_lte('Y', 'U', 'Y', 'V');
pub const ENCODING_YVYU: Fourcc = chars_to_fourcc_lte('Y', 'V', 'Y', 'U');
pub const ENCODING_UYVY: Fourcc = chars_to_fourcc_lte('U', 'Y', 'V', 'Y');
pub const ENCODING_VYUY: Fourcc = chars_to_fourcc_lte('V', 'Y', 'U', 'Y');
pub const ENCODING_NV12: Fourcc = chars_to_fourcc_lte('N', 'V', '1', '2');
pub const ENCODING_NV21: Fourcc = chars_to_fourcc_lte('N', 'V', '2', '1');
pub const ENCODING_ARGB: Fourcc = chars_to_fourcc_lte('A', 'R', 'G', 'B');
pub const ENCODING_ARGB_SLICE: Fourcc = chars_to_fourcc_lte('a', 'r', 'g', 'b');
pub const ENCODING_RGBA: Fourcc = chars_to_fourcc_lte('R', 'G', 'B', 'A');
pub const ENCODING_RGBA_SLICE: Fourcc = chars_to_fourcc_lte('r', 'g', 'b', 'a');
pub const ENCODING_ABGR: Fourcc = chars_to_fourcc_lte('A', 'B', 'G', 'R');
pub const ENCODING_ABGR_SLICE: Fourcc = chars_to_fourcc_lte('a', 'b', 'g', 'r');
pub const ENCODING_BGRA: Fourcc = chars_to_fourcc_lte('B', 'G', 'R', 'A');
pub const ENCODING_BGRA_SLICE: Fourcc = chars_to_fourcc_lte('b', 'g', 'r', 'a');
pub const ENCODING_RGB16: Fourcc = chars_to_fourcc_lte('R', 'G', 'B', '2');
pub const ENCODING_RGB16_SLICE: Fourcc = chars_to_fourcc_lte('r', 'g', 'b', '2');
pub const ENCODING_RGB24: Fourcc = chars_to_fourcc_lte('R', 'G', 'B', '3');
pub const ENCODING_RGB24_SLICE: Fourcc = chars_to_fourcc_lte('r', 'g', 'b', '3');
pub const ENCODING_RGB32: Fourcc = chars_to_fourcc_lte('R', 'G', 'B', '4');
pub const ENCODING_RGB32_SLICE: Fourcc = chars_to_fourcc_lte('r', 'g', 'b', '4');
pub const ENCODING_BGR16: Fourcc = chars_to_fourcc_lte('B', 'G', 'R', '2');
pub const ENCODING_BGR16_SLICE: Fourcc = chars_to_fourcc_lte('b', 'g', 'r', '2');
pub const ENCODING_BGR24: Fourcc = chars_to_fourcc_lte('B', 'G', 'R', '3');
pub const ENCODING_BGR24_SLICE: Fourcc = chars_to_fourcc_lte('b', 'g', 'r', '3');
pub const ENCODING_BGR32: Fourcc = chars_to_fourcc_lte('B', 'G', 'R', '4');
pub const ENCODING_BGR32_SLICE: Fourcc = chars_to_fourcc_lte('b', 'g', 'r', '4');
