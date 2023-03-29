use std::convert::TryFrom;

use log::trace;
use nom::{
    bytes::streaming::take,
    combinator::{complete, flat_map, map, map_parser, opt, verify},
    number::streaming::{be_f32, be_f64},
    sequence::{pair, preceded},
    Err, IResult, Needed, Parser,
};

use crate::permutation::matroska_permutation;

#[derive(Debug, PartialEq, Eq)]
pub struct Error<'a> {
    input: &'a [u8],
    kind: ErrorKind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Nom(nom::error::ErrorKind),
    Custom(EbmlError),
}

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EbmlError {
    /// The value of an unsigned integer does not fit into the platform's
    /// native uint type. This can not happen on 64-bit platforms.
    UintTooLarge = 0,

    /// A required value was not found by the parser.
    MissingRequiredValue = 1,

    /// One of the segment element types was discovered more than once in the input.
    DuplicateSegment = 2,

    /// The VINT_WIDTH is 8 or more, which means that the resulting variable-size
    /// integer is more than 8 octets wide. This is currently not supported.
    VintTooWide = 100,

    /// A signed integer element has declared a length of more than 8 octets,
    /// which is not allowed.
    IntTooWide = 101,

    /// An unsigned integer element has declared a length of more than 8 octets,
    /// which is not allowed.
    UintTooWide = 102,

    /// A float element has declared a length that is not 0, 4 or 8 octets,
    /// which is not allowed.
    FloatWidthIncorrect = 103,

    /// A string element contains non-UTF-8 data, which is not allowed.
    StringNotUtf8 = 104,
}

impl<'a> nom::error::ParseError<&'a [u8]> for Error<'a> {
    fn from_error_kind(input: &'a [u8], kind: nom::error::ErrorKind) -> Self {
        Error {
            input,
            kind: ErrorKind::Nom(kind),
        }
    }

    fn append(_input: &'a [u8], _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }
}

pub fn custom_error(input: &[u8], code: EbmlError) -> nom::Err<Error> {
    nom::Err::Error(Error {
        input,
        kind: ErrorKind::Custom(code),
    })
}

pub(crate) fn usize_error(input: &[u8], size: u64) -> Result<usize, nom::Err<Error>> {
    usize::try_from(size).map_err(|_| {
        log::error!("Not possible to convert size into usize");
        custom_error(input, EbmlError::UintTooLarge)
    })
}

pub(crate) fn value_error<T>(input: &[u8], value: Option<T>) -> Result<T, nom::Err<Error>> {
    value.ok_or_else(|| {
        log::error!("Not possible to get the requested value");
        custom_error(input, EbmlError::MissingRequiredValue)
    })
}

pub fn vint(input: &[u8]) -> IResult<&[u8], u64, Error> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return Err(custom_error(input, EbmlError::VintTooWide));
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

// The ID are represented in the specification as their binary representation
// do not drop the marker bit.
pub fn vid(input: &[u8]) -> IResult<&[u8], u64, Error> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return Err(custom_error(input, EbmlError::VintTooWide));
    }

    if input.len() <= len as usize {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let mut val = u64::from(v);

    trace!("vid {:08b} {:08b} {:08b} {}", val, v, (1 << (8 - len)), len);

    for i in 0..len as usize {
        val = (val << 8) | u64::from(input[i + 1]);
    }

    trace!("     result {:08x}", val);

    Ok((&input[len as usize + 1..], val))
}

pub fn parse_uint_data(size: u64) -> impl Fn(&[u8]) -> IResult<&[u8], u64, Error> {
    move |input| {
        let mut val = 0;

        if size > 8 {
            return Err(custom_error(input, EbmlError::UintTooWide));
        }

        for i in input.iter().take(size as usize) {
            val = (val << 8) | u64::from(*i);
        }

        Ok((&input[size as usize..], val))
    }
}

pub fn parse_int_data(size: u64) -> impl Fn(&[u8]) -> IResult<&[u8], i64, Error> {
    move |input| {
        let mut val = 0;

        if size > 8 {
            return Err(custom_error(input, EbmlError::IntTooWide));
        }

        for i in input.iter().take(size as usize) {
            val = (val << 8) | u64::from(*i);
        }

        Ok((&input[size as usize..], val as i64))
    }
}

pub fn parse_str_data(size: u64) -> impl Fn(&[u8]) -> IResult<&[u8], String, Error> {
    move |input| {
        take(usize_error(input, size)?)(input).and_then(|(i, data)| {
            match String::from_utf8(data.to_owned()) {
                Ok(s) => Ok((i, s)),
                Err(_) => return Err(custom_error(i, EbmlError::StringNotUtf8)),
            }
        })
    }
}

pub fn parse_binary_data(size: u64) -> impl Fn(&[u8]) -> IResult<&[u8], Vec<u8>, Error> {
    move |input| {
        map(take(usize_error(input, size)?), |data: &[u8]| {
            data.to_owned()
        })(input)
    }
}

pub fn parse_binary_data_ref(size: u64) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8], Error> {
    move |input| map(take(usize_error(input, size)?), |data| data)(input)
}

//FIXME: handle default values
//FIXME: is that really following IEEE_754-1985 ?
pub fn parse_float_data(size: u64) -> impl Fn(&[u8]) -> IResult<&[u8], f64, Error> {
    move |input| {
        if size == 0 {
            Ok((input, 0f64))
        } else if size == 4 {
            map(map_parser(take(4usize), be_f32), f64::from)(input)
        } else if size == 8 {
            map_parser(take(8usize), be_f64)(input)
        } else {
            Err(custom_error(input, EbmlError::FloatWidthIncorrect))
        }
    }
}

fn compute_ebml_type<'a, G, H, O1>(
    id: u64,
    second: G,
) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], O1, Error>
where
    G: Fn(u64) -> H,
    H: Parser<&'a [u8], O1, Error<'a>>,
{
    move |i| {
        flat_map(pair(verify(vid, |val| *val == id), vint), |(_, size)| {
            second(size)
        })(i)
    }
}

pub fn ebml_uint<'a>(id: u64) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], u64, Error> {
    compute_ebml_type(id, parse_uint_data)
}

pub fn ebml_int<'a>(id: u64) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], i64, Error> {
    compute_ebml_type(id, parse_int_data)
}

pub fn ebml_float<'a>(id: u64) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], f64, Error> {
    compute_ebml_type(id, parse_float_data)
}

pub fn ebml_str<'a>(id: u64) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], String, Error> {
    compute_ebml_type(id, parse_str_data)
}

pub fn ebml_binary<'a>(id: u64) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], Vec<u8>, Error> {
    compute_ebml_type(id, parse_binary_data)
}

pub fn ebml_binary_ref<'a>(id: u64) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], &'a [u8], Error> {
    compute_ebml_type(id, parse_binary_data_ref)
}

pub fn ebml_master<'a, G, O1>(
    id: u64,
    second: G,
) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], O1, Error>
where
    G: Fn(&'a [u8]) -> IResult<&'a [u8], O1, Error> + Copy,
{
    move |i| {
        pair(verify(vid, |val| *val == id), vint)(i)
            .and_then(|(i, (_, size))| map_parser(take(usize_error(i, size)?), second)(i))
    }
}

pub fn eat_void<'a, G, O1>(second: G) -> impl Fn(&'a [u8]) -> IResult<&'a [u8], O1, Error<'a>>
where
    G: Parser<&'a [u8], O1, Error<'a>> + Copy,
{
    move |i| preceded(opt(skip_void), second)(i)
}

pub fn skip_void(input: &[u8]) -> IResult<&[u8], &[u8], Error> {
    pair(verify(vid, |val| *val == 0xEC), vint)(input)
        .and_then(|(i, (_, size))| take(usize_error(input, size)?)(i))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EBMLHeader {
    pub version: u64,
    pub read_version: u64,
    pub max_id_length: u64,
    pub max_size_length: u64,
    pub doc_type: String,
    pub doc_type_version: u64,
    pub doc_type_read_version: u64,
}

pub fn ebml_header(input: &[u8]) -> IResult<&[u8], EBMLHeader, Error> {
    ebml_master(0x1A45DFA3, |i| {
        matroska_permutation((
            complete(ebml_uint(0x4286)), // version
            complete(ebml_uint(0x42F7)), // read_version
            complete(ebml_uint(0x42F2)), // max id length
            complete(ebml_uint(0x42F3)), // max size length
            complete(ebml_str(0x4282)),  // doctype
            complete(ebml_uint(0x4287)), // doctype version
            complete(ebml_uint(0x4285)), // doctype_read version
        ))(i)
        .and_then(|(i, t)| {
            Ok((
                i,
                EBMLHeader {
                    version: value_error(input, t.0)?,
                    read_version: value_error(input, t.1)?,
                    max_id_length: value_error(input, t.2)?,
                    max_size_length: value_error(input, t.3)?,
                    doc_type: value_error(input, t.4)?,
                    doc_type_version: value_error(input, t.5)?,
                    doc_type_read_version: value_error(input, t.6)?,
                },
            ))
        })
    })(input)
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use log::trace;
    use nom::{HexDisplay, Offset};

    use super::*;

    const single_stream: &[u8] = include_bytes!("../assets/single_stream.mkv");
    const webm: &[u8] = include_bytes!("../assets/big-buck-bunny_trailer.webm");

    #[test]
    fn variable_integer() {
        let val01 = [0b10000000];

        match vint(&val01) {
            Ok((_, v)) => assert!(0 == v),
            _ => panic!(),
        }
    }

    #[test]
    fn mkv_header() {
        trace!("{}", single_stream[..8].to_hex(8));
        trace!("{:b} {:b}", single_stream[0], single_stream[1]);
        trace!("{:?}", ebml_header(&single_stream[..100]).unwrap());
    }

    #[test]
    fn webm_header() {
        trace!("{}", webm[..8].to_hex(8));
        let res = ebml_header(&webm[..100]);
        trace!("{:?}", res);

        if let Ok((i, _)) = res {
            trace!("offset: {} bytes", webm.offset(i));
        } else {
            panic!();
        }
    }
}
