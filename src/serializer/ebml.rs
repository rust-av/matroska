use cookie_factory::gen::set_be_u8;
use cookie_factory::gen_slice;
use cookie_factory::GenError;
use nom::AsBytes;

use crate::ebml::{Date, EbmlHeader};
use crate::serializer::cookie_utils::{
    gen_at_offset, gen_skip, gen_slice, set_be_f64, set_be_i8, tuple,
};

const ALLOWED_ID_VALUES: u64 = (1u64 << 56) - 1;

pub(crate) fn vint_size(i: u64) -> Result<u8, GenError> {
    if i >= ALLOWED_ID_VALUES {
        return Err(GenError::CustomError(0));
    }

    let mut val = 1;

    loop {
        if ((i + 1) >> (val * 7)) == 0 {
            break;
        }

        val += 1;
    }
    Ok(val)
}

// Will return zero if i == 0.
pub(crate) fn vid_size(i: u32) -> u8 {
    4 - (i.leading_zeros() / 8) as u8
}

pub(crate) fn gen_vint(
    num: u64,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    move |mut input| {
        let needed_bytes = vint_size(num)?;

        let num = num | 1u64 << (needed_bytes * 7);

        let mut i = needed_bytes - 1;
        loop {
            match set_be_u8((input.0, input.1), (num >> (i * 8)) as u8) {
                Ok(next) => {
                    input = next;
                }
                Err(e) => return Err(e),
            }

            if i == 0 {
                break;
            }
            i -= 1;
        }

        Ok(input)
    }
}

pub(crate) fn gen_vid(
    num: u32,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    move |input| {
        let skip = 4 - vid_size(num) as usize;
        gen_slice!(input, &num.to_be_bytes()[skip..])
    }
}

pub(crate) fn gen_uint(
    num: u64,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    move |mut input| {
        let needed_bytes = vint_size(num)?;

        let mut i = needed_bytes - 1;
        loop {
            match set_be_u8((input.0, input.1), (num.wrapping_shr((i * 8).into())) as u8) {
                Ok(next) => {
                    input = next;
                }
                Err(e) => return Err(e),
            }

            if i == 0 {
                break;
            }
            i -= 1;
        }

        Ok(input)
    }
}

//FIXME: is it the right implementation?
pub(crate) fn gen_int(
    num: i64,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    move |mut input| {
        let needed_bytes = vint_size(num as u64)?;

        let mut i = needed_bytes - 1;
        loop {
            match set_be_i8((input.0, input.1), (num >> (i * 8)) as i8) {
                Ok(next) => {
                    input = next;
                }
                Err(e) => return Err(e),
            }

            if i == 0 {
                break;
            }
            i -= 1;
        }

        Ok(input)
    }
}

fn gen_type<T: Copy, G>(
    id: u32,
    size: u64,
    num: T,
    f: G,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError>
where
    G: Fn((&mut [u8], usize), T) -> Result<(&mut [u8], usize), GenError>,
{
    move |input| {
        let temp = gen_vid(id)(input)?;
        let temp = gen_vint(size)(temp)?;
        f(temp, num)
    }
}

pub(crate) fn gen_f64(
    id: u32,
    num: f64,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    move |input| gen_type(id, 8, num, set_be_f64)(input)
}

pub(crate) fn gen_ebml_size(
    expected_size: u8,
    size: usize,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    move |input| {
        let v = vint_size(size as u64)?;

        if v > expected_size {
            Err(GenError::CustomError(0))
        } else {
            gen_vint(size as u64)(input)
        }
    }
}

pub(crate) fn gen_ebml_master<'a, 'b, G>(
    id: u32,
    expected_size: u8,
    f: G,
) -> impl Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a
where
    G: Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a,
{
    move |input| {
        let (buf, ofs_len) = gen_vid(id)(input)?;
        let (buf, start) = gen_skip(expected_size as usize)((buf, ofs_len))?;
        let (buf, end) = f((buf, start))?;
        gen_at_offset(ofs_len, gen_ebml_size(expected_size, end - start))((buf, end))
    }
}

pub(crate) fn gen_ebml_uint_l<G>(
    id: u32,
    num: u64,
    expected_size: G,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError>
where
    G: Fn() -> Result<u8, GenError>,
{
    move |input| {
        let expected_size = expected_size()?;
        let needed_bytes = vint_size(expected_size as u64)?;

        let (buf, ofs_len) = gen_vid(id)(input)?;
        let (buf, start) = gen_skip(needed_bytes as usize)((buf, ofs_len))?;
        let (buf, end) = gen_uint(num)((buf, start))?;
        gen_at_offset(ofs_len, gen_ebml_size(expected_size, end - start))((buf, end))
    }
}

pub(crate) fn gen_ebml_uint(
    id: u32,
    num: u64,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    gen_ebml_uint_l(id, num, move || vint_size(num))
}

pub(crate) fn gen_ebml_int(
    id: u32,
    num: i64,
) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
    move |input| {
        let expected_size = vint_size(num as u64)?;
        let needed_bytes = vint_size(expected_size as u64)?;

        let (buf, ofs_len) = gen_vid(id)(input)?;
        let (buf, start) = gen_skip(needed_bytes as usize)((buf, ofs_len))?;
        let (buf, end) = gen_int(num)((buf, start))?;
        gen_at_offset(ofs_len, gen_ebml_size(expected_size, end - start))((buf, end))
    }
}

pub(crate) fn gen_ebml_str<'a, 'b>(
    id: u32,
    s: &'a str,
) -> impl Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a {
    move |input| {
        let v = vint_size(s.len() as u64)?;

        let (buf, ofs_len) = gen_vid(id)(input)?;
        let (buf, start) = gen_skip(v as usize)((buf, ofs_len))?;
        let (buf, end) = gen_slice(s.as_bytes())((buf, start))?;
        gen_at_offset(ofs_len, gen_ebml_size(v, end - start))((buf, end))
    }
}

pub(crate) fn gen_ebml_binary<'a, 'b, S>(
    id: u32,
    s: S,
) -> impl Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a
where
    S: AsRef<[u8]> + 'a,
{
    move |input| {
        let s = s.as_ref();
        let v = vint_size(s.len() as u64)?;

        let (buf, ofs_len) = gen_vid(id)(input)?;
        let (buf, start) = gen_skip(v as usize)((buf, ofs_len))?;
        let (buf, end) = gen_slice(s)((buf, start))?;
        gen_at_offset(ofs_len, gen_ebml_size(v, end - start))((buf, end))
    }
}

impl EbmlSize for EbmlHeader {
    fn capacity(&self) -> usize {
        self.version.size(0x4286)
            + self.read_version.size(0x42F7)
            + self.max_id_length.size(0x42F2)
            + self.max_size_length.size(0x42F3)
            + self.doc_type.size(0x4282)
            + self.doc_type_version.size(0x4287)
            + self.doc_type_read_version.size(0x4285)
    }
}

pub(crate) fn gen_ebml_header<'a, 'b>(
    h: &'a EbmlHeader,
) -> impl Fn((&'b mut [u8], usize)) -> Result<(&'b mut [u8], usize), GenError> + 'a {
    move |input| {
        gen_ebml_master(
            0x1A45DFA3,
            vint_size(h.capacity() as u64)?,
            tuple((
                gen_ebml_uint_l(0x4286, h.version as u64, || Ok(1)),
                gen_ebml_uint_l(0x42F7, h.read_version as u64, || Ok(1)),
                gen_ebml_uint_l(0x42F2, h.max_id_length as u64, || Ok(1)),
                gen_ebml_uint_l(0x42F3, h.max_size_length as u64, || Ok(1)),
                gen_ebml_str(0x4282, &h.doc_type),
                gen_ebml_uint_l(0x4287, h.doc_type_version as u64, || Ok(1)),
                gen_ebml_uint_l(0x4285, h.doc_type_read_version as u64, || Ok(1)),
            )),
        )(input)
    }
}

pub trait EbmlSize {
    fn capacity(&self) -> usize;

    fn size(&self, id: u32) -> usize {
        let id_size = vid_size(id);
        let self_size = self.capacity();
        let size_tag_size = vint_size(self_size as u64).unwrap_or(0);

        id_size as usize + size_tag_size as usize + self_size
    }
}

impl EbmlSize for u32 {
    fn capacity(&self) -> usize {
        (*self as u64).capacity()
    }
}

impl EbmlSize for u64 {
    fn capacity(&self) -> usize {
        vint_size(*self).unwrap_or(0) as usize
    }
}

impl EbmlSize for i64 {
    fn capacity(&self) -> usize {
        vint_size(*self as u64).unwrap_or(0) as usize
    }
}

impl EbmlSize for f64 {
    fn capacity(&self) -> usize {
        //FIXME: calculate size
        8
    }
}

impl EbmlSize for Date {
    fn capacity(&self) -> usize {
        // FIXME: Handle zero-sized date
        8
    }
}

impl<T: EbmlSize> EbmlSize for Option<T> {
    fn capacity(&self) -> usize {
        match *self {
            Some(ref value) => value.capacity(),
            None => 0,
        }
    }

    fn size(&self, id: u32) -> usize {
        match *self {
            Some(_) => {
                let id_size = vid_size(id);
                let self_size = self.capacity();
                let size_tag_size = vint_size(self_size as u64).unwrap_or(0);

                id_size as usize + size_tag_size as usize + self_size
            }
            None => 0,
        }
    }
}

impl EbmlSize for String {
    fn capacity(&self) -> usize {
        self.as_bytes().len()
    }
}

impl EbmlSize for Vec<u8> {
    fn capacity(&self) -> usize {
        (*self).as_bytes().len()
    }
}

impl<const N: usize> EbmlSize for [u8; N] {
    fn capacity(&self) -> usize {
        N
    }
}

impl<'a> EbmlSize for &'a [u8] {
    fn capacity(&self) -> usize {
        self.len()
    }
}

impl<'a> EbmlSize for Vec<&'a [u8]> {
    fn capacity(&self) -> usize {
        self.iter().fold(0, |acc, sl| acc + sl.len())
    }
}

impl EbmlSize for Vec<u64> {
    fn capacity(&self) -> usize {
        self.len() * 8
    }
}

impl EbmlSize for uuid::Uuid {
    fn capacity(&self) -> usize {
        16
    }
}

#[cfg(test)]
mod tests {
    use cookie_factory::gen::set_be_u64;
    use nom::HexDisplay;
    use quickcheck::quickcheck;

    use crate::ebml::EbmlResult;

    use super::*;

    fn gen_u64(
        id: u32,
        num: u64,
    ) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
        move |input| gen_type(id, 8, num, set_be_u64)(input)
    }

    fn gen_u8(
        id: u32,
        num: u8,
    ) -> impl Fn((&mut [u8], usize)) -> Result<(&mut [u8], usize), GenError> {
        move |input| gen_type(id, 1, num, set_be_u8)(input)
    }

    fn test_vint_serializer(i: u64) -> bool {
        println!("testing for {i}");

        let mut data = [0u8; 10];
        {
            let gen_res = gen_vint(i)((&mut data[..], 0));
            if let Err(e) = gen_res {
                println!("gen_res is error: {e:?}");
                println!("Large id value: {i:?}");
                // Do not fail if quickcheck generated data is too large
                return i >= ALLOWED_ID_VALUES;
            }
            println!("gen_res: {gen_res:?}");
        }
        println!("{}", (data[..]).to_hex(16));

        let parse_res = crate::ebml::vint(&data[..]);
        println!("parse_res: {parse_res:?}");
        match parse_res {
            Ok((_rest, o)) => {
                assert_eq!(i, o);
                true
            }
            e => panic!("parse error: {e:?}"),
        }
    }

    quickcheck! {
      fn test_vint(i: u64) -> bool {
        test_vint_serializer(i)
      }
    }

    #[test]
    fn vint() {
        test_vint_serializer(0);
        test_vint_serializer(8);
        test_vint_serializer(127);
        test_vint_serializer(128);
        test_vint_serializer(2100000);
    }

    fn test_vid_serializer(id: u32) -> bool {
        println!("\ntesting for id={id}");

        let mut data = [0u8; 10];
        {
            let gen_res = gen_vid(id)((&mut data[..], 0));
            println!("gen_res: {gen_res:?}");
        }
        println!("{}", (data[..]).to_hex(16));

        let parse_res = crate::ebml::vid(&data[..]);
        println!("parse_res: {parse_res:?}");
        match parse_res {
            Ok((_rest, o)) => {
                println!("id={id:08b}, parsed={o:08b}");
                assert_eq!(id, o);
                true
            }
            e => panic!("parse error: {e:?}"),
        }
    }

    #[test]
    fn vid() {
        test_vid_serializer(128);
        test_vid_serializer(0x4286);
        test_vid_serializer(0x1A45DFA3);
    }

    fn test_ebml_u64_serializer(num: u64) -> bool {
        let id = 0x9F;
        println!("\ntesting for id={id}, num={num}");

        let data = &mut [0u8; 100];
        {
            let (_, written) = gen_u64(id, num)((data, 0)).unwrap();
            println!("written: {written:?}");
        }
        println!("{}", data.to_hex(16));

        let parse_res: EbmlResult<u64> = crate::ebml::ebml_element(id)(data);
        println!("parse_res: {parse_res:?}");
        match parse_res {
            Ok((_rest, o)) => {
                assert_eq!(num, o);
                true
            }
            e => panic!("parse error: {e:?}"),
        }
    }

    quickcheck! {
      fn test_ebml_u64(i: u64) -> bool {
        test_ebml_u64_serializer(i)
      }
    }

    #[test]
    fn ebml_u64() {
        test_ebml_u64_serializer(0);
        test_ebml_u64_serializer(8);
        test_ebml_u64_serializer(127);
        test_ebml_u64_serializer(128);
        test_ebml_u64_serializer(2100000);
    }

    quickcheck! {
      fn test_ebml_u8(num: u8) -> bool {
        let id = 0x9F;
        println!("testing for id={id}, num={num}");

        let mut data = [0u8; 100];
        {
          let gen_res = gen_u8(id, num)((&mut data[..], 0));
          println!("gen_res: {gen_res:?}");
        }
        println!("{}", (data[..]).to_hex(16));

        let parse_res: EbmlResult<u64> = crate::ebml::ebml_element(id)(&data[..]);
        println!("parse_res: {parse_res:?}");
        match parse_res {
          Ok((_rest, o)) => {
            assert_eq!(num as u64, o);
            true
          },
          e => panic!("parse error: {e:?}"),
        }
      }
    }

    quickcheck! {
      fn test_ebml_header(version: u8, read_version: u8, max_id_length: u8, max_size_length: u8, doc_type: String,
        doc_type_version: u8, doc_type_read_version: u8) -> bool {
        let header = EbmlHeader {
          version: version as u32,
          read_version: read_version as u32,
          max_id_length: max_id_length as u32,
          max_size_length: max_size_length as u32,
          doc_type,
          doc_type_version: doc_type_version as u32,
          doc_type_read_version: doc_type_read_version as u32,
        };

        println!("will serialize: {header:#?}");
        let mut data = [0u8; 100];
        {
          let gen_res = gen_ebml_header(&header)((&mut data[..], 0));
          println!("gen_res: {gen_res:?}");
          // Do not fail if quickcheck generated data is too large
          match gen_res {
            Err(GenError::BufferTooSmall(_)) => return true,
            Err(_) => return false,
            Ok(_)  => (),
          }
        }

        println!("{}", (data[..]).to_hex(16));
        let parse_res = crate::ebml::ebml_header(&data[..]);
        println!("parse_res: {parse_res:?}");
        match parse_res {
          Ok((_rest, h)) => {
            assert_eq!(header, h);
            true
          },
          e => panic!("parse error: {e:?}"),
        }
      }
    }
}
