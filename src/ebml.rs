use std::convert::TryFrom;

use log::trace;
use nom::{
    branch::permutation,
    bytes::streaming::take,
    combinator::{complete, flat_map, map, map_parser, opt, verify},
    number::streaming::{be_f32, be_f64},
    sequence::{pair, preceded},
    Err, IResult, Needed, Parser,
};

/*
struct Document {
    header: Header,
    body: Vec<Element>,
}


struct Header {}

#[derive(Debug)]
enum ElementData {
    Signed(i64),
    Unsigned(u64),
    Float(f64),
    PlainString(String),
    UTF8String(String),
    Date(u64),
    Master(Vec<Element>),
    Binary(Vec<u8>),
    Unknown(u64),
}

#[derive(Debug)]
pub struct Element {
    id: u64,
    size: u64,
    data: ElementData,
}
*/

#[derive(Debug, PartialEq)]
pub struct Error<'a> {
    input: &'a [u8],
    kind: ErrorKind,
}

#[derive(Debug, PartialEq)]
pub enum ErrorKind {
    Nom(nom::error::ErrorKind),
    Custom(u8),
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

pub fn custom_error(input: &[u8], code: u8) -> Error {
    Error {
        input,
        kind: ErrorKind::Custom(code),
    }
}

pub(crate) fn usize_error(input: &[u8], size: u64) -> Result<usize, nom::Err<Error>> {
    usize::try_from(size).map_err(|_| {
        log::error!("Not possible to convert size into usize");
        nom::Err::Error(custom_error(input, 0))
    })
}

pub fn vint(input: &[u8]) -> IResult<&[u8], u64, Error> {
    if input.is_empty() {
        return Err(Err::Incomplete(Needed::new(1)));
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return Err(Err::Error(custom_error(input, 100)));
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
        return Err(Err::Error(custom_error(input, 100)));
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
            return Err(Err::Error(custom_error(input, 102)));
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
            return Err(Err::Error(custom_error(input, 103)));
        }

        for i in input.iter().take(size as usize) {
            val = (val << 8) | u64::from(*i);
        }

        Ok((&input[size as usize..], val as i64))
    }
}

pub fn parse_str_data(size: u64) -> impl Fn(&[u8]) -> IResult<&[u8], String, Error> {
    move |input| {
        map(take(usize_error(input, size)?), |data: &[u8]| {
            String::from_utf8(data.to_owned()).unwrap_or_default()
        })(input)
    }
}

pub fn parse_binary_data(size: u64) -> impl Fn(&[u8]) -> IResult<&[u8], Vec<u8>, Error> {
    move |input| {
        map(take(usize_error(input, size)?), |data: &[u8]| {
            data.to_owned()
        })(input)
    }
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
            Err(Err::Error(custom_error(input, 104)))
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

pub fn ebml_binary_ref(id: u64) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8], Error> {
    move |i| {
        pair(verify(vid, |val| *val == id), vint)(i)
            .and_then(|(i, (_, size))| take(usize_error(i, size)?)(i))
    }
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

#[derive(Debug, Clone, PartialEq)]
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
        map(
            permutation((
                ebml_uint(0x4286), // version
                ebml_uint(0x42F7), // read_version
                ebml_uint(0x42F2), // max id length
                ebml_uint(0x42F3), // max size length
                ebml_str(0x4282),  // doctype
                ebml_uint(0x4287), // doctype version
                ebml_uint(0x4285), // doctype_read version
                opt(complete(skip_void)),
            )),
            |t| EBMLHeader {
                version: t.0,
                read_version: t.1,
                max_id_length: t.2,
                max_size_length: t.3,
                doc_type: t.4,
                doc_type_version: t.5,
                doc_type_read_version: t.6,
            },
        )(i)
    })(input)
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use super::*;
    use log::trace;
    use nom::{HexDisplay, Offset};

    const single_stream: &'static [u8] = include_bytes!("../assets/single_stream.mkv");
    const webm: &'static [u8] = include_bytes!("../assets/big-buck-bunny_trailer.webm");

    #[test]
    fn variable_integer() {
        let val01 = [0b10000000];
        //        let val01 = [0b01000000, 0b1];

        match vint(&val01) {
            Ok((_, v)) => assert!(0 == v),
            _ => panic!(),
        }
    }

    /*
    #[test]
    fn header() {
        trace!("{}", single_stream[..8].to_hex(8));
        trace!("{:b} {:b}", single_stream[0], single_stream[1]);
        trace!("{:#?}", parse_element(single_stream));
        panic!();
    }*/

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
