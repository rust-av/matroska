use std::{
    convert::TryFrom,
    ops::{BitOr, Shl},
};

use crc::{Algorithm, Crc};
use log::trace;
use nom::{
    bytes::streaming::take,
    combinator::{complete, flat_map, map, map_parser, map_res, opt, verify},
    sequence::{preceded, tuple},
    Err, Needed, Parser,
};
use uuid::Uuid;

use crate::permutation::matroska_permutation;

pub(crate) type EbmlResult<'a, T> = nom::IResult<&'a [u8], T, Error>;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// nom returned an error.
    Nom(nom::error::ErrorKind),

    /// nom did not return an error, but the EBML is incorrect.
    /// The contained [u32] is the Element ID of the Element where the
    /// error occurred, or 0 if not applicable.
    ///
    /// For an overview of Element IDs, see the list of
    /// [EBML Element IDs] or [Matroska Element IDs].
    ///
    /// [EBML Element IDs]: https://www.rfc-editor.org/rfc/rfc8794.html#name-ebml-element-ids-registry
    /// [Matroska Element IDs]: https://www.ietf.org/archive/id/draft-ietf-cellar-matroska-15.html#section-27.1-11
    Ebml(u32, ParseError),
}

/// Describes what went wrong.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseError {
    /// The Element Data Size did not fit within a [usize].
    /// The current parsing code cannot handle an element of this size.
    ElementTooLarge,

    /// A required value was not found by the parser.
    MissingRequiredValue,

    /// One of the segment element types was discovered more than once in the input.
    DuplicateSegment,

    /// The VINT_WIDTH is 8 or more, which means that the resulting variable-size
    /// integer is more than 8 octets wide. This is currently not supported.
    VintTooWide,

    /// The VINT_WIDTH of this Element ID is 4 or more, which is not allowed as
    /// per the Matroska specification (Element IDs can be 1 to 4 octets long,
    /// except for the EBML Header which is also limited to 4 octets here).
    IDTooWide,

    /// A signed integer element has declared a length of more than 8 octets,
    /// which is not allowed.
    IntTooWide,

    /// An unsigned integer with a maximum length of 4 octets has declared a
    /// length of more than 4 octets, which is not allowed.
    U32TooWide,

    /// An unsigned integer element has declared a length of more than 8 octets,
    /// which is not allowed.
    UintTooWide,

    /// A float element has declared a length that is not 0, 4 or 8 octets,
    /// which is not allowed.
    FloatWidthIncorrect,

    /// A string element contains non-UTF-8 data, which is not allowed.
    StringNotUtf8,

    /// A binary element does not adhere to the length declared in the
    /// specification. The enclosed [u16] is the actual length of the data.
    BinaryWidthIncorrect(u16),

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

impl<I> nom::error::FromExternalError<I, Error> for Error {
    fn from_external_error(_input: I, _kind: nom::error::ErrorKind, e: Error) -> Self {
        e
    }
}

pub fn ebml_err<'a, T>(id: u32, err: ParseError) -> EbmlResult<'a, T> {
    Err(nom::Err::Error(Error::Ebml(id, err)))
}

pub(crate) fn value_error<T>(id: u32, value: Option<T>) -> Result<T, nom::Err<Error>> {
    value.ok_or_else(|| {
        log::error!("Not possible to get the requested value");
        nom::Err::Error(Error::Ebml(id, ParseError::MissingRequiredValue))
    })
}

pub fn vint(input: &[u8]) -> EbmlResult<u64> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return ebml_err(0, ParseError::VintTooWide);
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
            Error::Ebml(0, ParseError::ElementTooLarge)
        })
    })(input)
}

// The ID are represented in the specification as their binary representation
// do not drop the marker bit.
pub fn vid(input: &[u8]) -> EbmlResult<u32> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let len = 1 + input[0].leading_zeros() as usize;

    if input.len() <= len {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    match u32::try_parse(&input[..len]) {
        Ok(id) => Ok((&input[len..], id)),
        Err(_) => ebml_err(0, ParseError::IDTooWide),
    }
}

trait EbmlParsable: Sized {
    fn try_parse(data: &[u8]) -> Result<Self, ParseError>;
}

// Parsable implementation for the integer types
trait Int: From<u8> + Shl<Self, Output = Self> + BitOr<Self, Output = Self> {}
impl Int for u64 {}
impl Int for u32 {}
impl Int for i64 {}

impl<T: Int> EbmlParsable for T {
    fn try_parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() > std::mem::size_of::<T>() {
            return Err(ParseError::IntTooWide);
        }

        let mut val = Self::from(0);
        for b in data {
            val = (val << Self::from(8)) | Self::from(*b);
        }

        Ok(val)
    }
}

//FIXME: handle default values
//FIXME: is that really following IEEE_754-1985 ?
impl EbmlParsable for f64 {
    fn try_parse(data: &[u8]) -> Result<Self, ParseError> {
        match data.len() {
            0 => Ok(0.0),
            4 => Ok(f64::from(f32::from_be_bytes(data.try_into().unwrap()))),
            8 => Ok(f64::from_be_bytes(data.try_into().unwrap())),
            _ => Err(ParseError::FloatWidthIncorrect),
        }
    }
}

impl EbmlParsable for String {
    fn try_parse(data: &[u8]) -> Result<Self, ParseError> {
        String::from_utf8(data.to_vec()).map_err(|_| ParseError::StringNotUtf8)
    }
}

impl<const N: usize> EbmlParsable for [u8; N] {
    fn try_parse(data: &[u8]) -> Result<Self, ParseError> {
        let actual_len = data.len();
        data.try_into()
            .map_err(|_| ParseError::BinaryWidthIncorrect(actual_len as u16))
    }
}

impl EbmlParsable for Vec<u8> {
    fn try_parse(data: &[u8]) -> Result<Self, ParseError> {
        Ok(data.to_vec())
    }
}

impl EbmlParsable for Uuid {
    fn try_parse(data: &[u8]) -> Result<Self, ParseError> {
        <[u8; 16] as EbmlParsable>::try_parse(data).map(Uuid::from_bytes)
    }
}

fn ebml_generic<O: EbmlParsable>(id: u32) -> impl Fn(&[u8]) -> EbmlResult<O> {
    move |i| {
        let data = flat_map(preceded(verify(vid, |val| *val == id), elem_size), take);
        let parsed = map_res(data, |d| O::try_parse(d).map_err(|k| Error::Ebml(id, k)));
        complete(parsed)(i)
    }
}

pub fn u32(id: u32) -> impl Fn(&[u8]) -> EbmlResult<u32> {
    ebml_generic(id)
}

pub fn uint(id: u32) -> impl Fn(&[u8]) -> EbmlResult<u64> {
    ebml_generic(id)
}

pub fn int(id: u32) -> impl Fn(&[u8]) -> EbmlResult<i64> {
    ebml_generic(id)
}

pub fn float(id: u32) -> impl Fn(&[u8]) -> EbmlResult<f64> {
    ebml_generic(id)
}

pub fn str(id: u32) -> impl Fn(&[u8]) -> EbmlResult<String> {
    ebml_generic(id)
}

pub fn binary_exact<const N: usize>(id: u32) -> impl Fn(&[u8]) -> EbmlResult<[u8; N]> {
    ebml_generic(id)
}

pub fn binary(id: u32) -> impl Fn(&[u8]) -> EbmlResult<Vec<u8>> {
    ebml_generic(id)
}

pub fn uuid(id: u32) -> impl Fn(&[u8]) -> EbmlResult<Uuid> {
    ebml_generic(id)
}

// Doing this via EbmlParsable would make the trait more complicated,
// so it gets special treatment instead. This basically does the same
// thing as ebml_generic(id), but without a mapping function.
pub fn binary_ref<'a>(id: u32) -> impl Fn(&'a [u8]) -> EbmlResult<&'a [u8]> {
    move |i| {
        complete(flat_map(
            preceded(verify(vid, |val| *val == id), elem_size),
            take,
        ))(i)
    }
}

pub fn master<'a, F, O>(id: u32, second: F) -> impl Fn(&'a [u8]) -> EbmlResult<'a, O>
where
    F: Parser<&'a [u8], O, Error> + Copy,
{
    move |i| {
        tuple((verify(vid, |val| *val == id), elem_size, crc))(i).and_then(|(i, (_, size, crc))| {
            let size = if crc.is_some() { size - 6 } else { size };
            map_parser(checksum(crc, take(size)), second)(i)
        })
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
            Some(cs) if cs != CRC.checksum(o) => ebml_err(0, ParseError::Crc32Mismatch),
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
    master(0x1A45DFA3, |i| {
        matroska_permutation((
            u32(0x4286), // version
            u32(0x42F7), // read_version
            u32(0x42F2), // max id length
            u32(0x42F3), // max size length
            str(0x4282), // doctype
            u32(0x4287), // doctype version
            u32(0x4285), // doctype_read version
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
