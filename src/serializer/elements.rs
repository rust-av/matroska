use cookie_factory::*;
use elements::{Info, Seek, SeekHead, SegmentElement, Cluster};
use super::ebml::{vint_size, gen_vint, gen_vid, gen_uint};
use serializer::ebml::{gen_u64,gen_f64_ref};


pub fn seek_size(s: &Seek) -> u8 {
    // byte size of id (vid+size)+ data and position vid+size+int
    // FIXME: arbitrarily bad value
    vint_size(vint_size((s.id.len() + 10) as u64) as u64)
}

pub fn gen_segment<'a>(input: (&'a mut [u8], usize),
                       s: &SegmentElement)
                       -> Result<(&'a mut [u8], usize), GenError> {
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
    let capacity = s.positions.iter().fold(0u64, |acc, seek| {
        acc + 4 + 8 + 2 + vint_size(seek.id.len() as u64) as u64 + seek.id.len() as u64
    });
    println!("gen_seek_head: calculated capacity: {} -> {} bytes", capacity, vint_size(capacity));

    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
    0x114D9B74, byte_capacity,
    gen_many_ref!(&s.positions, gen_seek)
  )
}

pub fn gen_info<'a>(input: (&'a mut [u8], usize),
                         i: &Info)
                         -> Result<(&'a mut [u8], usize), GenError> {
    //FIXME: probably not large enough
    let capacity = 2 + i.segment_uid.as_ref().map(|s| s.len()).unwrap_or(0)
      + 2 + i.segment_filename.as_ref().map(|s| s.len()).unwrap_or(0)
      + 3 + i.prev_uid.as_ref().map(|s| s.len()).unwrap_or(0)
      + 3 + i.prev_filename.as_ref().map(|s| s.len()).unwrap_or(0)
      + 3 + i.next_uid.as_ref().map(|s| s.len()).unwrap_or(0)
      + 3 + i.next_filename.as_ref().map(|s| s.len()).unwrap_or(0)
      + 2 + i.segment_family.as_ref().map(|s| s.len()).unwrap_or(0)
      //FIXME;
      // chapter translate
      + 8
      + 8
      + 2 + i.date_utc.as_ref().map(|s| s.len()).unwrap_or(0)
      + 2 + i.title.as_ref().map(|s| s.len()).unwrap_or(0)
      + 2 + i.muxing_app.len()
      + 2 + i.writing_app.len();

    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
      0x1549A966, byte_capacity,
      do_gen!(
           gen_opt!( i.segment_uid, gen_ebml_binary!(0x73A4) )
        >> gen_opt!( i.segment_filename, gen_ebml_str!(0x7384) )
        >> gen_opt!( i.prev_uid, gen_ebml_binary!(0x3CB923) )
        >> gen_opt!( i.prev_filename, gen_ebml_str!(0x3C83AB) )
        >> gen_opt!( i.next_uid, gen_ebml_binary!(0x3EB923) )
        >> gen_opt!( i.next_filename, gen_ebml_str!(0x3E83BB) )
        >> gen_opt!( i.segment_family, gen_ebml_binary!(0x4444) )
        //>> gen_opt!( i.chapter_translate, gen_chapter_translate )
        >> gen_call!(gen_u64, 0x2AD7B1, i.timecode_scale)
        >> gen_opt!( i.duration, gen_call!(gen_f64_ref, 0x4489) )
        >> gen_opt!( i.date_utc, gen_ebml_binary!(0x4461) )
        >> gen_opt!( i.title, gen_ebml_str!(0x7BA9) )
        >> gen_ebml_str!(0x4D80, i.muxing_app)
        >> gen_ebml_str!(0x5741, i.writing_app)
      )
    )
}

#[macro_export]
macro_rules! my_gen_many (
    (($i:expr, $idx:expr), $l:expr, $f:ident) => (
        $l.into_iter().fold(
            Ok(($i,$idx)),
            |r,v| {
                match r {
                    Err(e) => Err(e),
                    Ok(x) => { $f(x, v) },
                }
            }
        )
    );
    (($i:expr, $idx:expr), $l:expr, $f:ident!( $($args:tt)* )) => (
        $l.into_iter().fold(
            Ok(($i,$idx)),
            |r,v| {
                match r {
                    Err(e) => Err(e),
                    Ok(x) => {
                      let (i, idx) = x;
                      $f!((i, idx), $($args)*, v)
                    },
                }
            }
        )
    );
);

pub fn gen_cluster<'a>(input: (&'a mut [u8], usize),
                         c: &Cluster)
                         -> Result<(&'a mut [u8], usize), GenError> {
    let capacity = 2 + 8
      // FIXME: serialize SilentTracks
      + 2 + 8
      + 2 + 8
      + 2 + c.simple_block.iter().fold(0, |acc, data| acc+ data.len())
      // FIXME serialize BlockGRoups
      // FIXME: serialize encrypted block
      ;


    let byte_capacity = vint_size(capacity as u64);
    gen_ebml_master!(input,
      0x1F43B675, byte_capacity,
      do_gen!(
           gen_ebml_uint!(0xE7, c.timecode)
        >> gen_opt!( c.position, gen_ebml_uint!(0xA7) )
        >> gen_opt!( c.prev_size, gen_ebml_uint!(0xAB) )
        >> my_gen_many!( &c.simple_block, gen_ebml_binary!( 0xA3 ) )
      )
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
            positions: seeks.iter()
                .cloned()
                .map(|(position, id)| {
                    Seek {
                        id: id,
                        position: position,
                    }
                })
                .collect(),
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
            Ok((rest, SegmentElement::SeekHead(o))) => {
                if should_fail {
                    println!("parser should have failed on input for {:?}", seek_head);
                    println!("{}", (&data[..]).to_hex(16));
                    return false;
                }

                assert_eq!(seek_head, o);
                return true;
            }
            e => {
                if should_fail {
                    return true;
                }

                panic!(format!("parse error: {:?} for input: {:?}", e, seeks))
            }
        }

        false
    }

    quickcheck! {
    fn test_seek_head(seeks: Vec<(u64, Vec<u8>)>) -> bool {
      test_seek_head_serializer(seeks)
    }
  }
}
