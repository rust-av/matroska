use std::convert::TryFrom;

use crc::{Algorithm, Crc};
use log::trace;
use nom::{
    bytes::streaming::take,
    combinator::{complete, flat_map, map, map_parser, map_res, opt, verify},
    number::streaming::{be_f32, be_f64},
    sequence::{pair, preceded, tuple},
    Err, Needed, Parser,
};

use crate::permutation::matroska_permutation;

pub(crate) type EbmlResult<'a, T> = nom::IResult<&'a [u8], T, Error>;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// nom returned an error.
    Nom(nom::error::ErrorKind),

    /// nom did not return an error, but the EBML is incorrect.
    Ebml(ErrorKind),
}

// TODO: Add Element IDs (u64) to more of these variants

/// The [u64] contained in some of these error variants represents the
/// EBML or Matroska Element ID of the element where the error occurred.
///
/// For an overview of all Element IDs, see:
///
/// https://www.rfc-editor.org/rfc/rfc8794.html#name-ebml-element-ids-registry
///
/// https://www.ietf.org/archive/id/draft-ietf-cellar-matroska-15.html#section-27.1-11
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The Element Data Size did not fit within a [usize].
    /// The current parsing code cannot handle an element of this size.
    ElementTooLarge,

    /// A required value was not found by the parser.
    MissingRequiredValue(u32),

    /// One of the segment element types was discovered more than once in the input.
    DuplicateSegment(u64),

    /// The VINT_WIDTH is 8 or more, which means that the resulting variable-size
    /// integer is more than 8 octets wide. This is currently not supported.
    VintTooWide,

    /// The VINT_WIDTH of this Element ID is 4 or more, which is not allowed as
    /// per the Matroska specification (Element IDs can be 1 to 4 octets long,
    /// except for the EBML Header which is also limited to 4 octets here).
    IDTooWide,

    /// A signed integer element has declared a length of more than 8 octets,
    /// which is not allowed.
    IntTooWide(u32),

    /// An unsigned integer with a maximum length of 4 octets has declared a
    /// length of more than 4 octets, which is not allowed.
    U32TooWide(u32),

    /// An unsigned integer element has declared a length of more than 8 octets,
    /// which is not allowed.
    UintTooWide(u32),

    /// A float element has declared a length that is not 0, 4 or 8 octets,
    /// which is not allowed.
    FloatWidthIncorrect(u32),

    /// A string element contains non-UTF-8 data, which is not allowed.
    StringNotUtf8(u32),

    /// A binary element does not adhere to the length declared in the
    /// specification.
    BinaryWidthIncorrect(usize, u32),

    /// A CRC-32 element was found, but the checksum did not match.
    Crc32Mismatch,
}

impl<'a> nom::error::ParseError<&'a [u8]> for Error {
    fn from_error_kind(_input: &'a [u8], kind: nom::error::ErrorKind) -> Self {
        Error::Nom(kind)
    }

    fn append(_input: &'a [u8], _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

pub fn ebml_err<'a, T>(err: ErrorKind) -> EbmlResult<'a, T> {
    Err(nom::Err::Error(Error::Ebml(err)))
}

impl<I> nom::error::FromExternalError<I, Error> for Error {
    fn from_external_error(_input: I, _kind: nom::error::ErrorKind, e: Error) -> Self {
        e
    }
}

pub(crate) fn value_error<T>(id: u32, value: Option<T>) -> Result<T, nom::Err<Error>> {
    value.ok_or_else(|| {
        log::error!("Not possible to get the requested value");
        nom::Err::Error(Error::Ebml(ErrorKind::MissingRequiredValue(id)))
    })
}

pub fn vint(input: &[u8]) -> EbmlResult<u64> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return ebml_err(ErrorKind::VintTooWide);
    }

    if input.len() <= len as usize {
        return Err(Err::Incomplete(Needed::new(1)));
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
            Error::Ebml(ErrorKind::ElementTooLarge)
        })
    })(input)
}

// The ID are represented in the specification as their binary representation
// do not drop the marker bit.
pub fn vid(input: &[u8]) -> EbmlResult<u32> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return ebml_err(ErrorKind::IDTooWide);
    }

    if input.len() <= len as usize {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let mut val = u32::from(v);

    trace!("vid {val:08b} {v:08b} {:08b} {len}", (1 << (8 - len)));

    for i in 0..len as usize {
        val = (val << 8) | u32::from(input[i + 1]);
    }

    trace!("     result {:08x}", val);

    Ok((&input[len as usize + 1..], val))
}

pub fn parse_u32_data(id: u32, size: usize) -> impl Fn(&[u8]) -> EbmlResult<u32> {
    move |input| {
        let mut val = 0;

        if size > 4 {
            return ebml_err(ErrorKind::U32TooWide(id));
        }

        for i in input.iter().take(size) {
            val = (val << 8) | u32::from(*i);
        }

        Ok((&input[size..], val))
    }
}

pub fn parse_uint_data(id: u32, size: usize) -> impl Fn(&[u8]) -> EbmlResult<u64> {
    move |input| {
        let mut val = 0;

        if size > 8 {
            return ebml_err(ErrorKind::UintTooWide(id));
        }

        for i in input.iter().take(size) {
            val = (val << 8) | u64::from(*i);
        }

        Ok((&input[size..], val))
    }
}

pub fn parse_int_data(id: u32, size: usize) -> impl Fn(&[u8]) -> EbmlResult<i64> {
    move |input| {
        let mut val = 0;

        if size > 8 {
            return ebml_err(ErrorKind::IntTooWide(id));
        }

        for i in input.iter().take(size) {
            val = (val << 8) | u64::from(*i);
        }

        Ok((&input[size..], val as i64))
    }
}

pub fn parse_str_data(id: u32, size: usize) -> impl Fn(&[u8]) -> EbmlResult<String> {
    move |input| {
        take(size)(input).and_then(|(i, data)| match String::from_utf8(data.to_owned()) {
            Ok(s) => Ok((i, s)),
            Err(_) => ebml_err(ErrorKind::StringNotUtf8(id)),
        })
    }
}

pub fn parse_binary_exact<const N: usize>(
    _id: u32,
    size: usize,
) -> impl Fn(&[u8]) -> EbmlResult<[u8; N]> {
    move |input| match map(take(size), <[u8; N]>::try_from)(input) {
        Ok((i, Ok(arr))) => Ok((i, arr)),
        Ok((_, Err(_))) => ebml_err(ErrorKind::BinaryWidthIncorrect(size, N as u32)),
        Err(e) => Err(e),
    }
}

pub fn parse_binary_data(_id: u32, size: usize) -> impl Fn(&[u8]) -> EbmlResult<Vec<u8>> {
    move |input| map(take(size), |data: &[u8]| data.to_owned())(input)
}

pub fn parse_binary_data_ref(_id: u32, size: usize) -> impl Fn(&[u8]) -> EbmlResult<&[u8]> {
    move |input| map(take(size), |data| data)(input)
}

//FIXME: handle default values
//FIXME: is that really following IEEE_754-1985 ?
pub fn parse_float_data(id: u32, size: usize) -> impl Fn(&[u8]) -> EbmlResult<f64> {
    move |input| {
        if size == 0 {
            Ok((input, 0f64))
        } else if size == 4 {
            map(map_parser(take(size), be_f32), f64::from)(input)
        } else if size == 8 {
            map_parser(take(size), be_f64)(input)
        } else {
            ebml_err(ErrorKind::FloatWidthIncorrect(id))
        }
    }
}

fn compute_ebml_type<'a, G, H, O1>(id: u32, second: G) -> impl Fn(&'a [u8]) -> EbmlResult<'a, O1>
where
    G: Fn(u32, usize) -> H,
    H: Parser<&'a [u8], O1, Error>,
{
    move |i| {
        flat_map(
            pair(verify(vid, |val| *val == id), elem_size),
            |(id, size)| second(id, size),
        )(i)
    }
}

pub fn ebml_u32<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, u32> {
    compute_ebml_type(id, parse_u32_data)
}

pub fn ebml_uint<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, u64> {
    compute_ebml_type(id, parse_uint_data)
}

pub fn ebml_int<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, i64> {
    compute_ebml_type(id, parse_int_data)
}

pub fn ebml_float<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, f64> {
    compute_ebml_type(id, parse_float_data)
}

pub fn ebml_str<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, String> {
    compute_ebml_type(id, parse_str_data)
}

pub fn ebml_binary_exact<'a, const N: usize>(
    id: u32,
) -> impl Fn(&'a [u8]) -> EbmlResult<'a, [u8; N]> {
    compute_ebml_type(id, parse_binary_exact)
}

pub fn ebml_binary<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, Vec<u8>> {
    compute_ebml_type(id, parse_binary_data)
}

pub fn ebml_binary_ref<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<'a, &'a [u8]> {
    compute_ebml_type(id, parse_binary_data_ref)
}

pub fn ebml_master<'a, G, O1>(id: u32, second: G) -> impl Fn(&'a [u8]) -> EbmlResult<'a, O1>
where
    G: Fn(&'a [u8]) -> EbmlResult<'a, O1> + Copy,
{
    move |i| {
        tuple((verify(vid, |val| *val == id), elem_size, crc))(i).and_then(|(i, (_, size, crc))| {
            let size = if crc.is_some() { size - 6 } else { size };
            map_parser(checksum(crc, take(size)), second)(i)
        })
    }
}

pub fn eat_void<'a, G, O1>(second: G) -> impl Fn(&'a [u8]) -> EbmlResult<'a, O1>
where
    G: Parser<&'a [u8], O1, Error> + Copy,
{
    move |i| preceded(opt(skip_void), second)(i)
}

pub fn skip_void(input: &[u8]) -> EbmlResult<&[u8]> {
    pair(verify(vid, |val| *val == 0xEC), elem_size)(input).and_then(|(i, (_, size))| take(size)(i))
}

const CRC: Crc<u32> = Crc::<u32>::new(&Algorithm {
    init: 0xFFFFFFFF,
    ..crc::CRC_32_ISO_HDLC
});

pub fn crc(input: &[u8]) -> EbmlResult<Option<u32>> {
    opt(map(ebml_binary_exact::<4>(0xBF), u32::from_le_bytes))(input)
}

pub fn checksum<'a, G>(crc: Option<u32>, inner: G) -> impl Fn(&'a [u8]) -> EbmlResult<'a, &'a [u8]>
where
    G: Fn(&'a [u8]) -> EbmlResult<'a, &'a [u8]>,
{
    move |input| {
        let (i, o) = inner(input)?;

        // FIXME: don't just return an error, the spec has well-defined CRC error handling
        match crc {
            Some(cs) if cs != CRC.checksum(o) => ebml_err(ErrorKind::Crc32Mismatch),
            _ => Ok((i, o)),
        }
    }
}

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
    ebml_master(0x1A45DFA3, |i| {
        matroska_permutation((
            complete(ebml_u32(0x4286)), // version
            complete(ebml_u32(0x42F7)), // read_version
            complete(ebml_u32(0x42F2)), // max id length
            complete(ebml_u32(0x42F3)), // max size length
            complete(ebml_str(0x4282)), // doctype
            complete(ebml_u32(0x4287)), // doctype version
            complete(ebml_u32(0x4285)), // doctype_read version
        ))(i)
        .and_then(|(i, t)| {
            Ok((
                i,
                EbmlHeader {
                    version: t.0.unwrap_or(1),
                    read_version: t.1.unwrap_or(1),
                    max_id_length: t.2.unwrap_or(4),
                    max_size_length: t.3.unwrap_or(8),
                    doc_type: value_error(0x4282, t.4)?,
                    doc_type_version: t.5.unwrap_or(1),
                    doc_type_read_version: t.6.unwrap_or(1),
                },
            ))
        })
    })(input)
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn mkv_header() {
        for (f, expected) in mkv_headers() {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join("matroska_test_w1_1")
                .join(f);

            let bytes = std::fs::read(path).expect("can read file");
            let (_, header) = ebml_header(&bytes).expect("can parse header");
            assert_eq!(header, expected);
        }
    }

    #[test]
    fn webm_header() {
        let f = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets")
            .join("big-buck-bunny_trailer.webm");

        let webm = std::fs::read(f).expect("can read file");

        let expected = EbmlHeader {
            doc_type: "webm".into(),
            doc_type_version: 1,
            doc_type_read_version: 1,
            ..default_header()
        };

        let (_, header) = ebml_header(&webm[..100]).unwrap();
        assert_eq!(header, expected);
    }

    #[test]
    fn variable_integer() {
        let val01 = [0b10000000];

        match vint(&val01) {
            Ok((_, v)) => assert!(0 == v),
            _ => panic!(),
        }
    }

    fn mkv_headers() -> Vec<(&'static str, EbmlHeader)> {
        vec![
            ("test1.mkv", default_header()), // basic
            ("test2.mkv", default_header()), // includes CRC-32
            (
                // some non-default values
                "test4.mkv",
                EbmlHeader {
                    doc_type_version: 1,
                    doc_type_read_version: 1,
                    ..default_header()
                },
            ),
        ]
    }

    fn default_header() -> EbmlHeader {
        EbmlHeader {
            version: 1,
            read_version: 1,
            max_id_length: 4,
            max_size_length: 8,
            doc_type: "matroska".into(),
            doc_type_version: 2,
            doc_type_read_version: 2,
        }
    }
}
