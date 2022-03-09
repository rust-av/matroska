use crate::ebml::EBMLHeader;
use cookie_factory::*;
use nom::AsBytes;

pub fn vint_size(i: u64) -> u8 {
    let mut val = 1;

    loop {
        if ((i + 1) >> (val * 7)) == 0 {
            break;
        }

        val += 1;
    }
    val
}

pub fn log2(i: u64) -> u32 {
    64 - (i | 1).leading_zeros()
}

pub fn vid_size(i: u64) -> u8 {
    ((log2(i + 1) - 1) / 7) as u8
}

pub fn gen_vint(
    mut input: (&mut [u8], usize),
    mut num: u64,
) -> Result<(&mut [u8], usize), GenError> {
    let needed_bytes = vint_size(num);

    assert!(num < (1u64 << 56) - 1);

    num |= 1u64 << (needed_bytes * 7);

    let mut i = needed_bytes - 1;
    loop {
        match gen_be_u8!(input, (num >> (i * 8)) as u8) {
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

pub fn gen_vid(mut input: (&mut [u8], usize), num: u64) -> Result<(&mut [u8], usize), GenError> {
    let needed_bytes = vid_size(num);

    let mut i = needed_bytes - 1;

    loop {
        match gen_be_u8!(input, (num >> (i * 8)) as u8) {
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

pub fn gen_uint(mut input: (&mut [u8], usize), num: u64) -> Result<(&mut [u8], usize), GenError> {
    let needed_bytes = vint_size(num);

    let mut i = needed_bytes - 1;
    loop {
        match gen_be_u8!(input, (num.wrapping_shr((i * 8).into())) as u8) {
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

//FIXME: is it the right implementation?
pub fn gen_int(mut input: (&mut [u8], usize), num: i64) -> Result<(&mut [u8], usize), GenError> {
    let needed_bytes = vint_size(num as u64);

    let mut i = needed_bytes - 1;
    loop {
        match gen_be_i8!(input, (num >> (i * 8)) as i8) {
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

/*
pub fn vid_size(id: u64) -> u8 {

}
*/

pub fn gen_u64(
    input: (&mut [u8], usize),
    id: u64,
    num: u64,
) -> Result<(&mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 8) >> gen_be_u64!(num)
    )
}

pub fn gen_u32(
    input: (&mut [u8], usize),
    id: u64,
    num: u32,
) -> Result<(&mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 4) >> gen_be_u32!(num)
    )
}

pub fn gen_u16(
    input: (&mut [u8], usize),
    id: u64,
    num: u16,
) -> Result<(&mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 2) >> gen_be_u16!(num)
    )
}

pub fn gen_u8(input: (&mut [u8], usize), id: u64, num: u8) -> Result<(&mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 1) >> gen_be_u8!(num)
    )
}

pub fn gen_i64(
    input: (&mut [u8], usize),
    id: u64,
    num: i64,
) -> Result<(&mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 8) >> gen_be_i64!(num)
    )
}

pub fn gen_i32(
    input: (&mut [u8], usize),
    id: u64,
    num: i32,
) -> Result<(&mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 4) >> gen_be_i32!(num)
    )
}

pub fn gen_i16(
    input: (&mut [u8], usize),
    id: u64,
    num: i16,
) -> Result<(&mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 2) >> gen_be_i16!(num)
    )
}

pub fn gen_i8(input: (&mut [u8], usize), id: u64, num: i8) -> Result<(&mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 1) >> gen_be_i8!(num)
    )
}

pub fn gen_f64(
    input: (&mut [u8], usize),
    id: u64,
    num: f64,
) -> Result<(&mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 8) >> gen_be_f64!(num)
    )
}

// Allow because this looks like a false positive
#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn gen_f64_ref<'a>(
    input: (&'a mut [u8], usize),
    id: u64,
    num: &f64,
) -> Result<(&'a mut [u8], usize), GenError> {
    do_gen!(
        input,
        gen_call!(gen_vid, id) >> gen_call!(gen_vint, 8) >> gen_be_f64!(*num)
    )
}

#[macro_export]
macro_rules! gen_ebml_size (
  (($i:expr, $idx:expr), $expected_size:expr, $size:expr) => ({
    let v = vint_size($size);
    //trace!("size: {}, vint_size: {}, expected_size: {}", $size, v, $expected_size);

    if v > $expected_size {
      Err(GenError::CustomError(0))
    } else {
      gen_call!(($i, $idx), gen_vint, $size)
    }
  })
);

#[macro_export]
macro_rules! gen_ebml_master (
  (($i:expr, $idx:expr), $id:expr, $expected_size:expr, $($rest:tt)*) => ({

    do_gen!(($i, $idx),
                  gen_call!(gen_vid, $id)
      >> ofs_len: gen_skip!($expected_size as usize)
      >> start:   do_gen!($($rest)*)
      >> end:     gen_at_offset!(ofs_len, gen_ebml_size!($expected_size, (end-start) as u64))
      //>> end:     gen_dbg!(gen_at_offset!(ofs_len, gen_call!(gen_vint, (end-start) as u64)))
      )
  });
  (($i:expr, $idx:expr), $id:expr, $($rest:tt)*) => (
    gen_ebml_master!(($i, $idx), $id, 8, $($rest)*)
  );
  ($input:expr, $id:expr, $expected_size:expr, $($rest:tt)*) => (
    gen_ebml_master!(($input.0, $input.1), $id, $expected_size, $($rest)*)
  );
  ($input:expr, $id:expr, $($rest:tt)*) => (
    gen_ebml_master!(($input.0, $input.1), $id, 8, $($rest)*)
  );
);

#[macro_export]
macro_rules! gen_ebml_uint (
  (($i:expr, $idx:expr), $id:expr, $num:expr, $expected_size:expr) => ({
    let needed_bytes = vint_size($expected_size as u64);

    do_gen!(($i, $idx),
                  gen_call!(gen_vid, $id)
      >> ofs_len: gen_skip!(needed_bytes as usize)
      >> start:   gen_call!(gen_uint, $num)
      >> end:     gen_at_offset!(ofs_len, gen_ebml_size!($expected_size, (end-start) as u64))
    )
  });
  (($i:expr, $idx:expr), $id:expr, $num:expr) => ({
    let v = vint_size($num);
    gen_ebml_uint!(($i, $idx), $id, $num, v)
  });
);

#[macro_export]
macro_rules! gen_ebml_int (
  (($i:expr, $idx:expr), $id:expr, $num:expr, $expected_size:expr) => ({
    use crate::serializer::ebml::gen_int;
    let needed_bytes = vint_size($expected_size as u64);

    do_gen!(($i, $idx),
                  gen_call!(gen_vid, $id)
      >> ofs_len: gen_skip!(needed_bytes as usize)
      >> start:   gen_call!(gen_int, $num)
      >> end:     gen_at_offset!(ofs_len, gen_ebml_size!($expected_size, (end-start) as u64))
    )
  });
  (($i:expr, $idx:expr), $id:expr, $num:expr) => ({
    let v = vint_size($num as u64);
    gen_ebml_int!(($i, $idx), $id, $num, v)
  });
);

#[macro_export]
macro_rules! gen_ebml_str (
  (($i:expr, $idx:expr), $id:expr, $s:expr) => ({
    let v = vint_size($s.len() as u64);

    do_gen!(($i, $idx),
                  gen_call!(gen_vid, $id)
      >> ofs_len: gen_skip!(v as usize)
      >> start:   gen_slice!(($s.as_bytes()))
      >> end:     gen_at_offset!(ofs_len, gen_ebml_size!(v, (end-start) as u64))
    )
  });
);

#[macro_export]
macro_rules! gen_ebml_binary (
  (($i:expr, $idx:expr), $id:expr, $s:expr) => ({
    let v = vint_size($s.len() as u64);

    do_gen!(($i, $idx),
                  gen_call!(gen_vid, $id)
      >> ofs_len: gen_skip!(v as usize)
      >> start:   gen_slice!($s)
      >> end:     gen_at_offset!(ofs_len, gen_ebml_size!(v, (end-start) as u64))
    )
  });
);

#[macro_export]
macro_rules! gen_opt (
  (($i:expr, $idx:expr), $val:expr, $submac:ident!(  )) => ({
    if let Some(ref val) = $val {
      $submac!(($i,$idx), val)
    } else {
      Ok(($i,$idx))
    }
  });
  (($i:expr, $idx:expr), $val:expr, $submac:ident!( $($args:tt),+ )) => ({
    if let Some(ref val) = $val {
      $submac!(($i,$idx), $($args),+ , val)
    } else {
      Ok(($i,$idx))
    }
  })
);

#[macro_export]
macro_rules! gen_opt_copy (
  (($i:expr, $idx:expr), $val:expr, $submac:ident!(  )) => ({
    if let Some(val) = $val {
      $submac!(($i,$idx), val)
    } else {
      Ok(($i,$idx))
    }
  });
  (($i:expr, $idx:expr), $val:expr, $submac:ident!( $($args:tt),+ )) => ({
    if let Some(val) = $val {
      $submac!(($i,$idx), $($args),+ , val)
    } else {
      Ok(($i,$idx))
    }
  })
);

#[macro_export]
macro_rules! gen_dbg (
  (($i:expr, $idx:expr), $submac:ident!( $($args:tt)*)) => ({
    gen_dbg!(__impl $i, $idx, $submac!($($args)*))
  });

  ($input:expr, $submac:ident!( $($args:tt)*)) => ({

    let (i, idx) = $input;
    gen_dbg!(__impl i, idx, $submac!($($args)*))
  });

  (__impl $i:expr, $idx:expr, $submac:ident!( $($args:tt)* )) => ({
    use nom::HexDisplay;
    use std::slice::from_raw_parts_mut;

    let (i, idx) = ($i, $idx);

    let p = i.as_ptr() as usize;
    let len = i.len();
    let res = ($submac!((i,idx), $($args)*)).map(|(j,idx2)| idx2 + (j.as_ptr() as usize) - p);

    match res {
      Ok(index) => {
        let sl = unsafe {
          from_raw_parts_mut(p as *mut u8, len)
        };
        log::trace!("gen_dbg {}->{}: {}:\n{}", idx, index, stringify!($submac!($($args)*)), (&sl[idx..index]).to_hex(16));
        Ok((sl, index))
      },
      Err(e) => Err(e),
    }
  });
);

impl EbmlSize for EBMLHeader {
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

//trace_macros!(true);
// Clippy thinks this function is too complicated, but it doesn't really make sense to split it up
#[allow(clippy::cognitive_complexity)]
pub fn gen_ebml_header<'a>(
    input: (&'a mut [u8], usize),
    h: &EBMLHeader,
) -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = h.capacity() as u64;

    gen_ebml_master!(
        input,
        0x1A45DFA3,
        vint_size(capacity),
        gen_ebml_uint!(0x4286, h.version, 1)
            >> gen_ebml_uint!(0x42F7, h.read_version, 1)
            >> gen_ebml_uint!(0x42F2, h.max_id_length, 1)
            >> gen_ebml_uint!(0x42F3, h.max_size_length, 1)
            >> gen_ebml_str!(0x4282, h.doc_type)
            >> gen_ebml_uint!(0x4287, h.doc_type_version, 1)
            >> gen_ebml_uint!(0x4285, h.doc_type_read_version, 1)
    )
}

pub fn gen_u64_a(input: (&mut [u8], usize), num: u64) -> Result<(&mut [u8], usize), GenError> {
    gen_dbg!((input.0, input.1), gen_be_u64!(num))
}

pub trait EbmlSize {
    fn capacity(&self) -> usize;

    fn size(&self, id: u64) -> usize {
        let id_size = vid_size(id);
        let self_size = self.capacity();
        let size_tag_size = vint_size(self_size as u64);

        id_size as usize + size_tag_size as usize + self_size as usize
    }
}

impl EbmlSize for u64 {
    fn capacity(&self) -> usize {
        vint_size(*self) as usize
    }
}

impl EbmlSize for i64 {
    fn capacity(&self) -> usize {
        vint_size(*self as u64) as usize
    }
}

impl EbmlSize for f64 {
    fn capacity(&self) -> usize {
        //FIXME: calculate size
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

    fn size(&self, id: u64) -> usize {
        match *self {
            Some(_) => {
                let id_size = vid_size(id);
                let self_size = self.capacity();
                let size_tag_size = vint_size(self_size as u64);

                id_size as usize + size_tag_size as usize + self_size as usize
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ebml::Error;
    use log::info;
    use nom::*;
    use quickcheck::quickcheck;

    fn test_vint_serializer(i: u64) -> bool {
        info!("testing for {}", i);

        let mut data = [0u8; 10];
        {
            let gen_res = gen_vint((&mut data[..], 0), i);
            info!("gen_res: {:?}", gen_res);
        }
        info!("{}", (&data[..]).to_hex(16));

        let parse_res = crate::ebml::vint(&data[..]);
        info!("parse_res: {:?}", parse_res);
        match parse_res {
            Ok((_rest, o)) => {
                assert_eq!(i, o);
                return true;
            }
            e => panic!("{}", format!("parse error: {:?}", e)),
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

    fn test_vid_serializer(id: u64) -> bool {
        info!("\ntesting for id={}", id);

        let mut data = [0u8; 10];
        {
            let gen_res = gen_vid((&mut data[..], 0), id);
            info!("gen_res: {:?}", gen_res);
        }
        info!("{}", (&data[..]).to_hex(16));

        let parse_res = crate::ebml::vid(&data[..]);
        info!("parse_res: {:?}", parse_res);
        match parse_res {
            Ok((_rest, o)) => {
                info!("id={:08b}, parsed={:08b}", id, o);
                assert_eq!(id, o);
                return true;
            }
            e => panic!("{}", format!("parse error: {:?}", e)),
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
        info!("\ntesting for id={}, num={}", id, num);

        let mut data = [0u8; 100];
        {
            let gen_res = gen_u64((&mut data[..], 0), id, num);
            info!("gen_res: {:?}", gen_res);
        }
        info!("{}", (&data[..]).to_hex(16));

        let parse_res: IResult<&[u8], u64, Error> = crate::ebml::ebml_uint(id)(&data[..]);
        info!("parse_res: {:?}", parse_res);
        match parse_res {
            Ok((_rest, o)) => {
                assert_eq!(num, o);
                return true;
            }
            e => panic!("{}", format!("parse error: {:?}", e)),
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
        info!("testing for id={}, num={}", id, num);

        let mut data = [0u8; 100];
        {
          let gen_res = gen_u8((&mut data[..], 0), id, num);
          info!("gen_res: {:?}", gen_res);
        }
        info!("{}", (&data[..]).to_hex(16));

        let parse_res: IResult<&[u8], u64, Error> = crate::ebml::ebml_uint(id)(&data[..]);
        info!("parse_res: {:?}", parse_res);
        match parse_res {
          Ok((_rest, o)) => {
            assert_eq!(num as u64, o);
            return true;
          },
          e => panic!("{}", format!("parse error: {:?}", e)),
        }
      }
    }

    quickcheck! {
      fn test_ebml_header(version: u8, read_version: u8, max_id_length: u8, max_size_length: u8, doc_type: String,
        doc_type_version: u8, doc_type_read_version: u8) -> bool {
        let header = EBMLHeader {
          version: version as u64,
          read_version: read_version as u64,
          max_id_length: max_id_length as u64,
          max_size_length: max_size_length as u64,
          doc_type: doc_type,
          doc_type_version: doc_type_version as u64,
          doc_type_read_version: doc_type_read_version as u64
        };

        info!("will serialize: {:#?}", header);
        let mut data = [0u8; 100];
        {
          let gen_res = gen_ebml_header((&mut data[..], 0), &header);
          info!("gen_res: {:?}", gen_res);
          // do not fail if quickcheck generated data that is too large
          match gen_res {
            Err(GenError::BufferTooSmall(_)) => return true,
            Err(_) => return false,
            Ok(_)  => (),
          }
        }

        info!("{}", (&data[..]).to_hex(16));
        let parse_res = crate::ebml::ebml_header(&data[..]);
        info!("parse_res: {:?}", parse_res);
        match parse_res {
          Ok((_rest, h)) => {
            assert_eq!(header, h);
            return true;
          },
          e => panic!("{}", format!("parse error: {:?}", e)),
        }
      }
    }
}
