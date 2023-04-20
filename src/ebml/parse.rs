use std::ops::{BitOr, Shl};

use crc::{Algorithm, Crc};
use log::trace;
use nom::{
    bytes::streaming::take,
    combinator::{complete, map, map_parser, map_res, opt},
    sequence::{preceded, tuple},
    Err::Incomplete,
    Needed, Parser,
};
use uuid::Uuid;

use super::error::{ebml_err, Error, ErrorKind};

pub type EbmlResult<'a, T> = nom::IResult<&'a [u8], T, Error>;

pub trait EbmlParsable<'a>: Sized {
    /// Whether to check for a CRC-32 Element and validate the checksum.
    fn has_crc() -> bool {
        false
    }

    fn try_parse(data: &'a [u8]) -> Result<Self, ErrorKind>;
}

// Parsable implementation for the integer types
trait Int: From<u8> + Shl<Self, Output = Self> + BitOr<Self, Output = Self> {}
impl Int for u64 {}
impl Int for u32 {}
impl Int for i64 {}

impl<'a, T: Int> EbmlParsable<'a> for T {
    fn try_parse(data: &'a [u8]) -> Result<Self, ErrorKind> {
        if data.len() > std::mem::size_of::<T>() {
            return Err(ErrorKind::IntTooWide);
        }

        let mut val = Self::from(0);
        for b in data {
            val = (val << Self::from(8)) | Self::from(*b);
        }

        Ok(val)
    }
}

impl<'a> EbmlParsable<'a> for f64 {
    fn try_parse(data: &'a [u8]) -> Result<Self, ErrorKind> {
        match data.len() {
            0 => Err(ErrorKind::EmptyFloat),
            4 => Ok(f64::from(f32::from_be_bytes(data.try_into().unwrap()))),
            8 => Ok(f64::from_be_bytes(data.try_into().unwrap())),
            _ => Err(ErrorKind::FloatWidthIncorrect),
        }
    }
}

impl<'a> EbmlParsable<'a> for String {
    fn try_parse(data: &'a [u8]) -> Result<Self, ErrorKind> {
        String::from_utf8(data.to_vec()).map_err(|_| ErrorKind::StringNotUtf8)
    }
}

impl<'a, const N: usize> EbmlParsable<'a> for [u8; N] {
    fn try_parse(data: &'a [u8]) -> Result<Self, ErrorKind> {
        let actual_len = data.len();
        data.try_into()
            .map_err(|_| ErrorKind::BinaryWidthIncorrect(actual_len as u16))
    }
}

impl<'a> EbmlParsable<'a> for Vec<u8> {
    fn try_parse(data: &'a [u8]) -> Result<Self, ErrorKind> {
        Ok(data.to_vec())
    }
}

impl<'a> EbmlParsable<'a> for &'a [u8] {
    fn try_parse(data: &'a [u8]) -> Result<Self, ErrorKind> {
        Ok(data)
    }
}

impl<'a> EbmlParsable<'a> for Uuid {
    fn try_parse(data: &'a [u8]) -> Result<Self, ErrorKind> {
        <[u8; 16] as EbmlParsable>::try_parse(data).map(Uuid::from_bytes)
    }
}

// FIXME: Better error handling (via append?)
pub fn get_required<T>(val: Option<T>, id: u32) -> Result<T, ErrorKind> {
    val.ok_or_else(|| {
        log::error!("Required Element {id:#0X} missing");
        ErrorKind::MissingElement
    })
}

pub fn ebml_element<'a, O: EbmlParsable<'a>>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, O> {
    move |i| {
        let (i, mut size) = complete(preceded(check_id(id), elem_size))(i)?;
        let (i, crc) = if O::has_crc() { crc(i)? } else { (i, None) };

        if crc.is_some() {
            // The CRC-32 Element is 6 bytes long,
            // and we already consumed them above.
            size -= 6;
        }

        let (i, data) = checksum(crc, complete(take(size)))(i)?;
        match O::try_parse(data) {
            Ok(o) => Ok((i, o)),
            Err(kind) => ebml_err(id, kind),
        }
    }
}

pub fn u32<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, u32> {
    ebml_element(id)
}

pub fn uint<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, u64> {
    ebml_element(id)
}

pub fn int<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, i64> {
    ebml_element(id)
}

// FIXME: Define and double-check float parsing behaviour in error cases
// FIXME: Also implement a test suite for that
pub fn float<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, f64> {
    ebml_element(id)
}

/// Handles missing and empty (0-octet) elements.
pub fn float_or<'a>(id: u32, default: f64) -> impl Fn(&'a [u8]) -> EbmlResult<'a, f64> {
    move |input| match ebml_element(id)(input) {
        Err(nom::Err::Error(Error {
            id: _,
            kind: ErrorKind::MissingElement | ErrorKind::EmptyFloat,
        })) => Ok((input, default)),
        rest => rest,
    }
}

pub fn str<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, String> {
    ebml_element(id)
}

pub fn binary_exact<'a, const N: usize>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, [u8; N]> {
    ebml_element(id)
}

pub fn binary<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, Vec<u8>> {
    ebml_element(id)
}

pub fn uuid<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, Uuid> {
    ebml_element(id)
}

// Doing this via EbmlParsable<'a>would make the trait more complicated,
// so it gets special treatment instead. This basically does the same
// thing as ebml_generic(id), but without a mapping function.
pub fn binary_ref<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, &'a [u8]> {
    ebml_element(id)
}

pub fn master<'a, F, O>(id: u32, second: F) -> impl Fn(&'a [u8]) -> EbmlResult<'a, O>
where
    F: Parser<&'a [u8], O, Error> + Copy,
{
    move |i| {
        tuple((check_id(id), elem_size, crc))(i).and_then(|(i, (_, size, crc))| {
            let size = if crc.is_some() { size - 6 } else { size };
            map_parser(checksum(crc, take(size)), second)(i)
        })
    }
}

pub fn check_id<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, u32> {
    move |input| {
        let (i, o) = vid(input)?;

        if id == o {
            Ok((i, o))
        } else {
            ebml_err(id, ErrorKind::MissingElement)
        }
    }
}

pub fn skip_void<'a, F, O>(second: F) -> impl FnMut(&'a [u8]) -> EbmlResult<'a, O>
where
    F: Parser<&'a [u8], O, Error> + Copy,
{
    preceded(opt(void), second)
}

pub fn void(input: &[u8]) -> EbmlResult<&[u8]> {
    binary_ref(0xEC)(input)
}

/// Consumes an entire Master Element, and returns the ID if successful.
pub fn skip_master(input: &[u8]) -> EbmlResult<u32> {
    let (i, (id, size, crc)) = tuple((vid, elem_size, crc))(input)?;
    let size = if crc.is_some() { size - 6 } else { size };
    checksum(crc, take(size))(i)?;
    Ok((i, id))
}

const CRC: Crc<u32> = Crc::<u32>::new(&Algorithm {
    init: 0xFFFFFFFF,
    ..crc::CRC_32_ISO_HDLC
});

pub fn crc(input: &[u8]) -> EbmlResult<Option<u32>> {
    opt(map(binary_exact::<4>(0xBF), u32::from_le_bytes))(input)
}

pub fn checksum<'a, F>(
    crc: Option<u32>,
    mut inner: F,
) -> impl FnMut(&'a [u8]) -> EbmlResult<'a, &'a [u8]>
where
    F: Parser<&'a [u8], &'a [u8], Error>,
{
    move |input| {
        let (i, o) = inner.parse(input)?;

        // FIXME: don't just return an error, the spec has well-defined CRC error handling
        match crc {
            Some(cs) if cs != CRC.checksum(o) => ebml_err(0, ErrorKind::Crc32Mismatch),
            _ => Ok((i, o)),
        }
    }
}

pub fn vint(input: &[u8]) -> EbmlResult<u64> {
    if input.is_empty() {
        return Err(Incomplete(Needed::new(1)));
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return ebml_err(0, ErrorKind::VintTooWide);
    }

    if input.len() <= len as usize {
        return Err(Incomplete(Needed::new(1)));
    }

    let mut val = u64::from(v ^ (1 << (7 - len)));

    trace!(
        "vint {:08b} {:08b} {:08b} {}",
        val,
        v,
        (1 << (8 - len)),
        len
    );

    for i in 0..len as usize {
        val = (val << 8) | u64::from(input[i + 1]);
    }

    trace!("     result {:08x}", val);

    Ok((&input[len as usize + 1..], val))
}

// The take combinator can only accept `usize`, so we need to make
// sure that the `vint` fits inside those bounds.
pub fn elem_size(input: &[u8]) -> EbmlResult<usize> {
    map_res(vint, |u| {
        usize::try_from(u).map_err(|_| {
            log::error!("Element Data Size does not fit into usize");
            Error {
                id: 0,
                kind: ErrorKind::ElementTooLarge,
            }
        })
    })(input)
}

// The ID are represented in the specification as their binary representation
// do not drop the marker bit.
pub fn vid(input: &[u8]) -> EbmlResult<u32> {
    if input.is_empty() {
        return Err(Incomplete(Needed::new(1)));
    }

    let len = 1 + input[0].leading_zeros() as usize;

    if input.len() <= len {
        return Err(Incomplete(Needed::new(1)));
    }

    match u32::try_parse(&input[..len]) {
        Ok(id) => Ok((&input[len..], id)),
        Err(_) => ebml_err(0, ErrorKind::IDTooWide),
    }
}