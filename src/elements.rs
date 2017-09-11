use ebml::{vid, vint};

#[derive(Debug,Clone,PartialEq)]
pub enum SegmentElement {
    SeekHead(SeekHead),
    Info(Info),
    Tracks(Tracks),
    Chapters(Chapters),
    Cluster(Cluster),
    Cues(Cues),
    Attachments(Attachments),
    Tags(Tags),
    Unknown(u64, Option<u64>),
}

// https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.3
named!(pub segment<(u64, Option<u64>)>,
  do_parse!(
    id:   verify!(vid, |val:u64| val == 0x18538067) >>
    size: opt!(vint)   >>
    (id, size)
  )
);

named!(skip,
       do_parse!(
    size: vint >> data: take!(size) >> (data)
  ));

// Segment, the root element, has id 0x18538067
named!(pub segment_element<SegmentElement>,
  switch!(vid,
    0x114D9B74 => do_parse!(
         size: vint
      >> head: flat_map!(take!(size as usize), dbg_dmp!(seek_head))
      >> (head)
    ) |

    unknown    => do_parse!(
        size: opt!(vint) >>
              cond!((size.is_some()), take!( (size.unwrap() as usize) )) >>
              (SegmentElement::Unknown(unknown, size))
      )
  )
);

#[derive(Debug,Clone,PartialEq)]
pub struct SeekHead {
    positions: Vec<Seek>,
}


//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.4
named!(pub seek_head<SegmentElement>,
  do_parse!(
    positions: many1!(seek) >>
    (SegmentElement::SeekHead(SeekHead {
      positions: positions,
    }))
  )
);


#[derive(Debug,Clone,PartialEq)]
pub struct Seek {
    id: Vec<u8>,
    position: u64,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.4
named!(pub seek<Seek>,
  ebml_master!(0x4DBB,
    do_parse!(
      t: permutation!(
        ebml_binary!(0x53AB), // SeekID
        ebml_uint!(0x53AC)    // SeekPosition
      ) >>
      (Seek {
        id:       t.0,
        position: t.1,
      })
    )
  )
);

#[derive(Debug,Clone,PartialEq)]
pub struct Info {}

#[derive(Debug,Clone,PartialEq)]
pub struct Tracks {}

#[derive(Debug,Clone,PartialEq)]
pub struct Chapters {}

#[derive(Debug,Clone,PartialEq)]
pub struct Cluster {}

#[derive(Debug,Clone,PartialEq)]
pub struct Cues {}

#[derive(Debug,Clone,PartialEq)]
pub struct Attachments {}

#[derive(Debug,Clone,PartialEq)]
pub struct Tags {}


#[cfg(test)]
mod tests {
    use super::*;
    use nom::{HexDisplay, IResult, Offset};
    use std::cmp::min;

    const mkv: &'static [u8] = include_bytes!("../assets/single_stream.mkv");


    #[test]
    fn segment_root() {
        let res = segment(&mkv[47..100]);
        println!("{:?}", res);

        if let IResult::Done(i, _) = res {
            println!("consumed {} bytes after header", (&mkv[47..]).offset(i));
        }

        panic!();
    }

    #[test]
    fn segment_elements() {
        let mut index: usize = 59;

        loop {
            let res = segment_element(&mkv[index..]);

            match res {
                IResult::Done(i, o) => {
                    let new_index = mkv.offset(i);
                    match o {
                        SegmentElement::Unknown(id, size) => {
                            println!("[{} -> {}] Unknown {{ id: 0x{:x}, size: {:?} }}",
                                     index,
                                     new_index,
                                     id,
                                     size);
                        }
                        o => {
                            println!("[{} -> {}] {:#?}", index, new_index, o);
                        }
                    };

                    index = new_index as usize;
                }
                e => {
                    let max_index = min(mkv.len(), index + 200);
                    println!("[{}] {:#?}:\n{}",
                             index,
                             e,
                             (&mkv[index..max_index]).to_hex(16));
                    break;
                }
            }
        }

        panic!();
    }
}
