use log::trace;
use nom::{Err, IResult, Needed};

/* nom5 note
 *
 * Temporary use of ErrorKind::Fix instead of Custom
 *
 * TODO: define good custom error
 *
*/

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

pub fn vint(input: &[u8]) -> IResult<&[u8], u64, Error> {
    if input.len() == 0 {
        return Err(Err::Incomplete(Needed::Size(1)));
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return Err(Err::Error(custom_error(input, 100)));
    }

    if input.len() <= len as usize {
        return Err(Err::Incomplete(Needed::Size(1)));
    }

    let mut val = (v ^ (1 << (7 - len))) as u64;

    trace!(
        "vint {:08b} {:08b} {:08b} {}",
        val,
        v,
        (1 << (8 - len)),
        len
    );

    for i in 0..len as usize {
        val = (val << 8) | input[i + 1] as u64;
    }

    trace!("     result {:08x}", val);

    Ok((&input[len as usize + 1..], val))
}

// The ID are represented in the specification as their binary representation
// do not drop the marker bit.
pub fn vid(input: &[u8]) -> IResult<&[u8], u64, Error> {
    if input.len() == 0 {
        return Err(Err::Incomplete(Needed::Size(1)));
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return Err(Err::Error(custom_error(input, 100)));
    }

    if input.len() <= len as usize {
        return Err(Err::Incomplete(Needed::Size(1)));
    }

    let mut val = v as u64;

    trace!("vid {:08b} {:08b} {:08b} {}", val, v, (1 << (8 - len)), len);

    for i in 0..len as usize {
        val = (val << 8) | input[i + 1] as u64;
    }

    trace!("     result {:08x}", val);

    Ok((&input[len as usize + 1..], val))
}

/*
fn parse_master(input: &[u8], _: u64) -> IResult<&[u8], ElementData> {
    map!(input,
         many0!(parse_element),
         |elem| ElementData::Master(elem))
}

fn parse_uint(input: &[u8], size: u64) -> IResult<&[u8], ElementData> {
    let mut val = 0;

    if size > 8 {
        return IResult::Error(ErrorKind::Custom(1));
    }

    for i in 0..size as usize {
        val = (val << 8) | input[i] as u64;
    }

    IResult::Done(&input[size as usize..], ElementData::Unsigned(val))
}
*/
pub fn parse_uint_data(input: &[u8], size: u64) -> IResult<&[u8], u64, Error> {
    let mut val = 0;

    if size > 8 {
        return Err(Err::Error(custom_error(input, 102)));
    }

    for i in 0..size as usize {
        val = (val << 8) | input[i] as u64;
    }

    Ok((&input[size as usize..], val))
}

pub fn parse_int_data(input: &[u8], size: u64) -> IResult<&[u8], i64, Error> {
    let mut val = 0;

    if size > 8 {
        return Err(Err::Error(custom_error(input, 103)));
    }

    for i in 0..size as usize {
        val = (val << 8) | input[i] as u64;
    }

    //FIXME: is that right?
    Ok((&input[size as usize..], val as i64))
}

/*
fn parse_str(input: &[u8], size: u64) -> IResult<&[u8], ElementData> {
    do_parse!(input,
        s: take_s!(size as usize) >>
        ( ElementData::PlainString(String::from_utf8(s.to_owned()).unwrap()) )
    )
}
*/
pub fn parse_str_data(input: &[u8], size: u64) -> IResult<&[u8], String, Error> {
    do_parse!(
        input,
        s: take!(size as usize) >> (String::from_utf8(s.to_owned()).unwrap()) //FIXME: maybe do not unwrap here
    )
}

pub fn parse_binary_data(input: &[u8], size: u64) -> IResult<&[u8], Vec<u8>, Error> {
    do_parse!(input, s: take!(size as usize) >> (s.to_owned()))
}

//FIXME: handle default values
//FIXME: is that really following IEEE_754-1985 ?
pub fn parse_float_data(input: &[u8], size: u64) -> IResult<&[u8], f64, Error> {
    use nom::number::streaming::{be_f32, be_f64};
    if size == 0 {
        Ok((input, 0f64))
    } else if size == 4 {
        map!(input, flat_map!(take!(4), be_f32), |val| val as f64)
    } else if size == 8 {
        flat_map!(input, take!(8), be_f64)
    } else {
        Err(Err::Error(custom_error(input, 104)))
    }
}
/*
fn parse_element_id(input: &[u8], id: u64, size: u64) -> IResult<&[u8], ElementData> {
    // trace!("id: 0x{:X} size: {}", id, size);
    if input.len() < size as usize {
        return IResult::Incomplete(Needed::Size(size as usize));
    }

    match id {
        0x1A45DFA3 => parse_master(input, size),
        0x4286 => parse_uint(input, size),
        0x42F7 => parse_uint(input, size),
        0x42F2 => parse_uint(input, size),
        0x42F3 => parse_uint(input, size),
        0x4282 => parse_str(input, size),
        0x4287 => parse_uint(input, size),
        0x4285 => parse_uint(input, size),
        _ => IResult::Done(&input[size as usize..], ElementData::Unknown(id)),
    }
}

named!(pub parse_element<Element>,
    do_parse!(
        id : vid >>
        size: vint >>
        data: call!(parse_element_id, id, size) >>
        (Element { id, size, data })
    )
);
*/
#[macro_export]
macro_rules! ebml_uint (
  ($i: expr, $id:expr) => ({
    use $crate::ebml::{vid, vint, parse_uint_data};
    do_parse!($i,
               verify!(vid, |val:&u64| *val == $id)
      >> size: vint
      >> data: call!(parse_uint_data, size)
      >> (data)
    )
  })
);

#[macro_export]
macro_rules! ebml_int (
  ($i: expr, $id:expr) => ({
    use $crate::ebml::{vid, vint, parse_int_data};
    do_parse!($i,
               verify!(vid, |val:&u64| *val == $id)
      >> size: vint
      >> data: call!(parse_int_data, size)
      >> (data)
    )
  })
);

#[macro_export]
macro_rules! ebml_float (
  ($i: expr, $id:expr) => ({
    use $crate::ebml::{vid, vint, parse_float_data};
    do_parse!($i,
               verify!(vid, |val:&u64| *val == $id)
      >> size: vint
      >> data: call!(parse_float_data, size)
      >> (data)
    )
  })
);

#[macro_export]
macro_rules! ebml_str (
  ($i: expr, $id:expr) => ({
    use $crate::ebml::{vid, vint, parse_str_data};

    do_parse!($i,
               verify!(vid, |val:&u64| *val == $id)
      >> size: vint
      >> data: call!(parse_str_data, size)
      >> (data)
    )
  })
);

#[macro_export]
macro_rules! ebml_binary (
  ($i: expr, $id:expr) => ({
    use $crate::ebml::{vid, vint, parse_binary_data};

    do_parse!($i,
               verify!(vid, |val:&u64| *val == $id)
      >> size: vint
      >> data: call!(parse_binary_data, size)
      >> (data)
    )
  })
);

#[macro_export]
macro_rules! ebml_binary_ref (
  ($i: expr, $id:expr) => ({
    use $crate::ebml::{vid, vint};

    do_parse!($i,
               verify!(vid, |val:&u64| *val == $id)
      >> size: vint
      >> data: take!(size)
      >> (data)
    )
  })
);

#[macro_export]
macro_rules! ebml_master (
  ($i: expr, $id:expr, $submac:ident!( $($args:tt)* )) => ({
    use $crate::ebml::{vid, vint};
    do_parse!($i,
               verify!(vid, |val:&u64| *val == $id)
      >> size: vint
      >> data: flat_map!(take!(size as usize), $submac!($($args)*))
      >> (data)
    )
  })
);

#[macro_export]
macro_rules! eat_void (
  ($i: expr, $submac:ident!( $($args:tt)* )) => ({
    preceded!($i,
      opt!($crate::ebml::skip_void),
      $submac!($($args)*)
    )
  });
  ($i: expr, $e:expr) => ({
    eat_void!($i, call!($e))
  });
);

named!(pub skip_void<&[u8], &[u8], Error>,
do_parse!(
        // NOM5: why?
        verify!(vid, |val:&u64| *val == 0xEC) >>
  size: vint >>
  data: take!(size) >>
  (data)
  ));

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

// named!(pub ebml_header<EBMLHeader>,
named!(pub ebml_header<&[u8], EBMLHeader, Error>,
  ebml_master!(0x1A45DFA3,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0x4286), // version
        ebml_uint!(0x42F7), // read_version
        ebml_uint!(0x42F2), // max id length
        ebml_uint!(0x42F3), // max size length
        ebml_str!(0x4282),  // doctype
        ebml_uint!(0x4287), // doctype version
        ebml_uint!(0x4285),  // doctype_read version
        complete!(skip_void)?
      ) >>
      ({
        EBMLHeader {
          version:               t.0,
          read_version:          t.1,
          max_id_length:         t.2,
          max_size_length:       t.3,
          doc_type:              t.4,
          doc_type_version:      t.5,
          doc_type_read_version: t.6,

        }
      })
    )
  )
);

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
