use cookie_factory::*;

pub fn vint_size(i: u64) -> u8 {
  let mut val = 1;

  loop {
    if ((i + 1) >> val * 7) == 0 {
      break;
    }

    val += 1;
  }
  val
}

pub fn log2(i: u64) -> u32 {
  let res = 64 - (i|1).leading_zeros();
  println!("log2({}) = {}", i, res);
  res
}

pub fn vid_size(i: u64) -> u8 {
  //((log2(i + 1) - 1) / 7 + 1) as u8
  ((log2(i + 1) - 1) / 7) as u8
}

pub fn gen_vint<'a>(mut input:(&'a mut [u8],usize), mut num: u64) -> Result<(&'a mut [u8],usize),GenError> {
  let needed_bytes = vint_size(num);
  println!("needed bytes: {}", needed_bytes);

  assert!(num < (1u64 << 56) - 1);

  num |= 1u64 << needed_bytes * 7;

  let mut i = needed_bytes - 1;
  loop {

    match gen_be_u8!(input, (num >> i * 8) as u8) {
      Ok(next) => {
        input = next;
      },
      Err(e) => return Err(e),
    }

    if i == 0 { break }
    i -= 1;
  }

  Ok(input)
}

pub fn gen_vid<'a>(mut input:(&'a mut [u8],usize), mut num: u64) -> Result<(&'a mut [u8],usize),GenError> {
  let needed_bytes = vid_size(num);
  println!("needed bytes: {}", needed_bytes);

  //num |= 1u64 << needed_bytes * 7;

  let index = 0;
  let mut i = needed_bytes - 1;

  loop {

    match gen_be_u8!(input, (num >> i * 8) as u8) {
      Ok(next) => {
        input = next;
      },
      Err(e) => return Err(e),
    }

    if i == 0 { break }
    i -= 1;
  }

  Ok(input)
}

/*
pub fn vid_size(id: u64) -> u8 {
  
}
*/

pub fn gen_u64<'a>(input:(&'a mut [u8],usize), id: u64, num: u64) -> Result<(&'a mut [u8],usize),GenError> {
  do_gen!(input,
    gen_call!(gen_vid, id) >>
    gen_call!(gen_vint, 8)  >>
    gen_be_u64!(num)
  )
}

pub fn gen_u8<'a>(input:(&'a mut [u8],usize), id: u64, num: u8) -> Result<(&'a mut [u8],usize),GenError> {
  do_gen!(input,
    gen_call!(gen_vid, id) >>
    gen_call!(gen_vint, 1)  >>
    gen_be_u8!(num)
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use nom::*;

  fn test_vint_serializer(i: u64) -> bool {
    println!("testing for {}", i);

    let mut data = [0u8; 10];
    {
      let gen_res = gen_vint((&mut data[..], 0), i);
      println!("gen_res: {:?}", gen_res);
    }
    println!("{}", (&data[..]).to_hex(16));

    let parse_res = ::ebml::vint(&data[..]);
    println!("parse_res: {:?}", parse_res);
    match parse_res {
      IResult::Done(rest, o) => {
        assert_eq!(i, o);
        return true;
      },
      e => panic!(format!("parse error: {:?}", e)),
    }

    false
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
    println!("\ntesting for id={}", id);

    let mut data = [0u8; 10];
    {
      let gen_res = gen_vid((&mut data[..], 0), id);
      println!("gen_res: {:?}", gen_res);
    }
    println!("{}", (&data[..]).to_hex(16));

    let parse_res = ::ebml::vid(&data[..]);
    println!("parse_res: {:?}", parse_res);
    match parse_res {
      IResult::Done(rest, o) => {
        println!("id={:08b}, parsed={:08b}", id, o);
        assert_eq!(id, o);
        return true;
      },
      e => panic!(format!("parse error: {:?}", e)),
    }

    false
  }

  #[test]
  fn vid() {
    test_vid_serializer(128);
    test_vid_serializer(0x4286);
    test_vid_serializer(0x1A45DFA3);
  }

  fn test_ebml_u64_serializer(num: u64) -> bool {
    let id = 0x9F;
    println!("\ntesting for id={}, num={}", id, num);

    let mut data = [0u8; 100];
    {
      let gen_res = gen_u64((&mut data[..], 0), id, num);
      println!("gen_res: {:?}", gen_res);
    }
    println!("{}", (&data[..]).to_hex(16));

    let parse_res = ebml_uint!(&data[..], id);
    println!("parse_res: {:?}", parse_res);
    match parse_res {
      IResult::Done(rest, o) => {
        assert_eq!(num, o);
        return true;
      },
      e => panic!(format!("parse error: {:?}", e)),
    }

    false
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
    println!("testing for id={}, num={}", id, num);

    let mut data = [0u8; 100];
    {
      let gen_res = gen_u8((&mut data[..], 0), id, num);
      println!("gen_res: {:?}", gen_res);
    }
    println!("{}", (&data[..]).to_hex(16));

    let parse_res = ebml_uint!(&data[..], id);
    println!("parse_res: {:?}", parse_res);
    match parse_res {
      IResult::Done(rest, o) => {
        assert_eq!(num as u64, o);
        return true;
      },
      e => panic!(format!("parse error: {:?}", e)),
    }

    false
  }
}

}
