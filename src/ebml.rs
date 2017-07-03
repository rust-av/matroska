
use nom::{HexDisplay, IResult, Needed, ErrorKind};

struct Document {
    header: Header,
    body: Vec<Element>
}


struct Header {

}

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
    Unknown(u64)
}

#[derive(Debug)]
pub struct Element {
    id: u64,
    size: u64,
    data: ElementData
}

fn vint(input: &[u8]) -> IResult<&[u8], u64> {
    if input.len() == 0 {
        return IResult::Incomplete(Needed::Size(1))
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return IResult::Error(ErrorKind::Custom(1));
    }

    if input.len() < len as usize {
        return IResult::Incomplete(Needed::Size(1));
    }

    let mut val = (v ^ (1 << (7 - len))) as u64;

    // println!("vint {:08b} {:08b} {:08b} {}", val, v, (1 << (8 - len)), len);

    for i in 0..len as usize {
        val = (val << 8) | input[i + 1] as u64;
    }

    IResult::Done(&input[len as usize + 1 ..], val)
}

// The ID are represented in the specification as their binary representation
// do not drop the marker bit.
fn vid(input: &[u8]) -> IResult<&[u8], u64> {
    if input.len() == 0 {
        return IResult::Incomplete(Needed::Size(1))
    }

    let v = input[0];
    let len = v.leading_zeros();

    if len == 8 {
        return IResult::Error(ErrorKind::Custom(1));
    }

    if input.len() < len as usize {
        return IResult::Incomplete(Needed::Size(1));
    }

    let mut val = v as u64;

    // println!("vint {:08b} {:08b} {:08b} {}", val, v, (1 << (8 - len)), len);

    for i in 0..len as usize {
        val = (val << 8) | input[i + 1] as u64;
    }

    IResult::Done(&input[len as usize + 1 ..], val)
}

fn parse_master(input: &[u8], size: u64) -> IResult<&[u8], ElementData> {
    map!(input,
        many0!(parse_element), |elem| ElementData::Master(elem)
    )
}

fn parse_uint(input: &[u8], size: u64) -> IResult<&[u8], ElementData> {
    let mut val = 0;

    if size > 8 {
        return IResult::Error(ErrorKind::Custom(1));
    }

    for i in 0..size as usize {
        val = (val << 8) | input[i] as u64;
    }

    IResult::Done(&input[size as usize ..], ElementData::Unsigned(val))
}

use std::str;

fn parse_str(input: &[u8], size: u64) -> IResult<&[u8], ElementData> {
    do_parse!(input,
        s: take_s!(size as usize) >>
        ( ElementData::PlainString(String::from_utf8(s.to_owned()).unwrap()) )
    )
}

fn parse_element_id(input: &[u8], id: u64, size: u64) -> IResult<&[u8], ElementData> {
    // println!("id: 0x{:X} size: {}", id, size);
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
        _ => IResult::Done(&input[size as usize ..], ElementData::Unknown(id))
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

#[cfg(test)]
mod tests {
    use super::*;

    const single_stream : &'static [u8] = include_bytes!("../assets/single_stream.mkv");

    #[test]
    fn variable_integer() {
        let val01 = [0b10000000];
//        let val01 = [0b01000000, 0b1];

        match vint(&val01) {
            IResult::Done(_, v) => assert!(0 == v),
            _ => panic!()
        }
    }

    #[test]
    fn header() {
        println!("{}", single_stream[..8].to_hex(8));
        println!("{:b} {:b}", single_stream[0], single_stream[1]);
        println!("{:#?}", parse_element(single_stream));
        panic!();

    }
}
