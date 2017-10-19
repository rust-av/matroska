use cookie_factory::*;
use elements::{Seek, SeekHead, SegmentElement};
use super::ebml::{vint_size, gen_vint, gen_vid, gen_uint};


pub fn seek_size(s: &Seek) -> u8 {
    // byte size of id (vid+size)+ data and position vid+size+int
    // FIXME: arbitrarily bad value
    vint_size(vint_size((s.id.len() + 10) as u64) as u64)
}

pub fn gen_segment<'a>(input: (&'a mut [u8], usize), s: &SegmentElement) -> Result<(&'a mut [u8], usize), GenError> {
  unimplemented!();
  /*do_gen!(input,
    gen_call!(gen_vid, 0x18538067) >>
    gen_call!(gen_vint, 4)
  )*/
}

pub fn gen_seek<'a>(input: (&'a mut [u8], usize),
                    s: &Seek)
                    -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = 8 + 2 + vint_size(s.id.len() as u64) as u64 + s.id.len() as u64;

    gen_ebml_master!(input,
    0x4DBB, vint_size(capacity),
    gen_ebml_binary!(0x53AB, s.id) >>
    gen_ebml_uint!(0x53AC, s.position, vint_size(s.position))
  )
}

pub fn gen_seek_head<'a>(input: (&'a mut [u8], usize),
                         s: &SeekHead)
                         -> Result<(&'a mut [u8], usize), GenError> {
  let capacity = s.positions.iter().fold(0u64, |acc, seek| acc + 4 + 8 + 2 + vint_size(seek.id.len() as u64) as u64 + seek.id.len() as u64);
  println!("gen_seek_head: calculated capacity: {} -> {} bytes", capacity, vint_size(capacity));

  let byte_capacity = vint_size(capacity as u64);
  gen_ebml_master!(input,
    0x114D9B74, byte_capacity,
    gen_many_ref!(&s.positions, gen_seek)
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use nom::*;
  use std::iter::repeat;

  fn test_seek_head_serializer(mut seeks: Vec<(u64, Vec<u8>)>) -> bool {
    println!("testing for {:?}", seeks);

    let mut should_fail = false;
    if seeks.len() == 0 {
      should_fail = true;
    }

    for &(_, ref id) in seeks.iter() {
      println!("id: {}", id.to_hex(16));
      if id.len() == 0 {
        println!("id is empty, returning");
        return true;
        //should_fail = true;
      }
    }

    if should_fail {
      println!("the parser should fail");
    }

    let capacity = seeks.iter().fold(0, |acc, &(_, ref v)| acc + 8 + v.len() + 100);
    println!("defining capacity as {}", capacity);

    let mut data = Vec::with_capacity(capacity);
    data.extend(repeat(0).take(capacity));

    let seek_head = SeekHead {
      positions: seeks.iter().cloned().map(|(position, id)| Seek { id, position }).collect()
    };

    let ser_res = {
      let gen_res = gen_seek_head((&mut data[..], 0), &seek_head);
      println!("gen_res: {:?}", gen_res);
      if let Err(e) = gen_res {
        println!("gen_res is error: {:?}", e);
        println!("should fail: {:?}", should_fail);
        return should_fail;
        /*if should_fail {
          println!("should fail");
          return true;
        }*/
      }
    };

    println!("ser_res: {:?}", ser_res);

    let parse_res = ::elements::segment_element(&data[..]);
    println!("parse_res: {:?}", parse_res);
    match parse_res {
      IResult::Done(rest, SegmentElement::SeekHead(o)) => {
        if should_fail {
          println!("parser should have failed on input for {:?}", seek_head);
          println!("{}", (&data[..]).to_hex(16));
          return false;
        }

        assert_eq!(seek_head, o);
        return true;
      },
      e => {
        if should_fail {
          return true;
        }

        panic!(format!("parse error: {:?} for input: {:?}", e, seeks))
      },
    }

    false
  }

  quickcheck! {
    fn test_seek_head(seeks: Vec<(u64, Vec<u8>)>) -> bool {
      test_seek_head_serializer(seeks)
    }
  }
}
