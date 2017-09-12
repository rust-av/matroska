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

#[macro_export]
macro_rules! sub_element(
  ($i:expr, $parser:ident) => ({
    do_parse!($i,
         size: vint
      >> element: flat_map!(take!(size as usize), dbg_dmp!($parser))
      >> (element)
    )
  })
);

// Segment, the root element, has id 0x18538067
named!(pub segment_element<SegmentElement>,
  switch!(vid,
    0x114D9B74 => sub_element!(seek_head) |
    0x1549A966 => sub_element!(info)      |

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
pub struct Info {
    pub segment_uid: Option<Vec<u8>>,
    pub segment_filename: Option<String>,
    pub prev_uid: Option<Vec<u8>>,
    pub prev_filename: Option<String>,
    pub next_uid: Option<Vec<u8>>,
    pub next_filename: Option<String>,
    pub segment_family: Option<Vec<u8>>,
    pub chapter_translate: Option<ChapterTranslate>,
    pub timecode_scale: u64,
    pub duration: Option<f64>, // FIXME should be float
    pub date_utc: Option<Vec<u8>>, //FIXME: should be date
    pub title: Option<String>,
    pub muxing_app: String,
    pub writing_app: String,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.8
named!(pub info<SegmentElement>,
  do_parse!(
    t: permutation_opt!(
      dbg_dmp!(complete!(ebml_binary!(0x73A4)))?, // SegmentUID
      dbg_dmp!(complete!(ebml_str!(0x7384)))?,    // SegmentFIlename FIXME SHOULD BE UTF-8 not str
      complete!(ebml_binary!(0x3CB923))?,         // PrevUID
      complete!(ebml_str!(0x3C83AB))?,            // PrevFilename FIXME SHOULD BE UTF-8 not str
      complete!(ebml_binary!(0x3EB923))?,         // NextUID
      complete!(ebml_str!(0x3E83BB))?,            // NextFilename FIXME SHOULD BE UTF-8 not str
      complete!(ebml_binary!(0x4444))?,           // SegmentFamily
      complete!(chapter_translate)?,              //
      complete!(ebml_uint!(0x2AD7B1)),            // TimecodeScale
      complete!(ebml_float!(0x4489))?,           // Duration: FIXME should be float
      complete!(ebml_binary!(0x4461))?,           // DateUTC FIXME: should be date
      complete!(ebml_str!(0x7BA9))?,              // Title FIXME SHOULD BE UTF-8 not str
      complete!(ebml_str!(0x4D80)),               // MuxingApp FIXME SHOULD BE UTF-8 not str
      complete!(ebml_str!(0x5741))                // WritingApp FIXME SHOULD BE UTF-8 not str
    ) >> (SegmentElement::Info(Info {
      segment_uid: t.0,
      segment_filename: t.1,
      prev_uid: t.2,
      prev_filename: t.3,
      next_uid: t.4,
      next_filename: t.5,
      segment_family: t.6,
      chapter_translate: t.7,
      timecode_scale:   t.8,
      duration: t.9,
      date_utc: t.10,
      title: t.11,
      muxing_app: t.12,
      writing_app: t.13
    }))
  )
);

#[derive(Debug,Clone,PartialEq)]
pub struct ChapterTranslate {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
//TODO
named!(pub chapter_translate<ChapterTranslate>,
  dbg_dmp!(ebml_master!(0x6924, value!(ChapterTranslate{})))
);

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
