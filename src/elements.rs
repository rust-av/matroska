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
    Void,
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
  });
  ($i:expr, $submac:ident!( $($args:tt)* )) => ({
    do_parse!($i,
         size: vint
      >> element: flat_map!(take!(size as usize), dbg_dmp!($submac!($($args)*)))
      >> (element)
    )
  });
);

// Segment, the root element, has id 0x18538067
named!(pub segment_element<SegmentElement>,
  switch!(vid,
      0x114D9B74 => sub_element!(seek_head)
    | 0x1549A966 => sub_element!(info)
    | 0x1F43B675 => sub_element!(cluster)
    | 0x1043A770 => sub_element!(chapters)
    | 0x1254C367 => sub_element!(value!(SegmentElement::Tags(Tags { })))
    | 0x1941A469 => sub_element!(value!(SegmentElement::Attachments(Attachments { })))
    | 0x1654AE6B => sub_element!(value!(SegmentElement::Tracks(Tracks { })))
    | 0x1C53BB6B => sub_element!(value!(SegmentElement::Cues(Cues { })))
    | 0xEC       => sub_element!(value!(SegmentElement::Void))
    | unknown    => do_parse!(
        size: opt!(vint) >>
              cond!(size.is_some(), take!( (size.unwrap() as usize) )) >>
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
      complete!(ebml_binary!(0x73A4))?, // SegmentUID
      complete!(ebml_str!(0x7384))?,    // SegmentFIlename FIXME SHOULD BE UTF-8 not str
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
  ebml_master!(0x6924, value!(ChapterTranslate{}))
);

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.26
#[derive(Debug,Clone,PartialEq)]
pub struct Cluster {
    pub timecode: u64,
    pub silent_tracks: Option<SilentTracks>,
    pub position: Option<u64>,
    pub prev_size: Option<u64>,
    pub simple_block: Option<Vec<u8>>,
    pub block_group: Option<BlockGroup>,
    pub encrypted_block: Option<Vec<u8>>,
}

named!(pub cluster<SegmentElement>,
  do_parse!(
    t: permutation_opt!(
      complete!(ebml_uint!(0xE7)),
      complete!(silent_tracks)?,
      complete!(ebml_uint!(0xA7))?,
      complete!(ebml_uint!(0xAB))?,
      complete!(ebml_binary!(0xA3))?,
      complete!(block_group)?,
      complete!(ebml_binary!(0xAF))?
    ) >> (SegmentElement::Cluster(Cluster {
      timecode: t.0,
      silent_tracks: t.1,
      position: t.2,
      prev_size: t.3,
      simple_block: t.4,
      block_group: t.5,
      encrypted_block: t.6,
    }))
  )
);

#[derive(Debug,Clone,PartialEq)]
pub struct SilentTracks {
    numbers: Vec<u64>,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
named!(pub silent_tracks<SilentTracks>,
  ebml_master!(0x5854, map!(many0!(ebml_uint!(0x58D7)), |v| SilentTracks { numbers: v }))
);

#[derive(Debug,Clone,PartialEq)]
pub struct BlockGroup {
    pub block: Vec<u8>,
    pub block_virtual: Option<Vec<u8>>,
    pub block_additions: Option<BlockAdditions>,
    pub block_duration: Option<u64>,
    pub reference_priority: u64,
    pub reference_block: Option<u64>,
    pub reference_virtual: Option<i64>,
    pub codec_state: Option<Vec<u8>>,
    pub discard_padding: Option<i64>,
    pub slices: Option<Slices>,
    pub reference_frame: Option<ReferenceFrame>,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
//TODO
named!(pub block_group<BlockGroup>,
  ebml_master!(0x5854,
    do_parse!(
      t: permutation_opt!(
        complete!(ebml_binary!(0xA1)),
        complete!(ebml_binary!(0xA2))?,
        complete!(block_additions)?,
        complete!(ebml_uint!(0x9B))?,
        complete!(ebml_uint!(0xFA)),
        complete!(ebml_uint!(0xFB))?,
        complete!(ebml_int!(0xFD))?,
        complete!(ebml_binary!(0xA4))?,
        complete!(ebml_int!(0x75A2))?,
        complete!(slices)?,
        complete!(reference_frame)?

      ) >> (BlockGroup {
        block: t.0,
        block_virtual: t.1,
        block_additions: t.2,
        block_duration: t.3,
        reference_priority: t.4,
        reference_block: t.5,
        reference_virtual: t.6,
        codec_state: t.7,
        discard_padding: t.8,
        slices: t.9,
        reference_frame: t.10
      })
    )
  )
);

#[derive(Debug,Clone,PartialEq)]
pub struct BlockAdditions {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
//TODO
named!(pub block_additions<BlockAdditions>,
  ebml_master!(0x75A1, value!(BlockAdditions {}))
);

#[derive(Debug,Clone,PartialEq)]
pub struct Slices {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.46
//TODO
named!(pub slices<Slices>,
  ebml_master!(0x8E, value!(Slices {}))
);

#[derive(Debug,Clone,PartialEq)]
pub struct ReferenceFrame {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.53
//TODO
named!(pub reference_frame<ReferenceFrame>,
  ebml_master!(0xC8, value!(ReferenceFrame {}))
);


#[derive(Debug,Clone,PartialEq)]
pub struct Tracks {}

#[derive(Debug,Clone,PartialEq)]
pub struct Chapters {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.199
//TODO
named!(pub chapters<SegmentElement>,
  //EditionEntry
  ebml_master!(0x45B9, value!(SegmentElement::Chapters(Chapters{})))
);

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
    const webm: &'static [u8] = include_bytes!("../assets/big-buck-bunny_trailer.webm");


    #[test]
    fn mkv_segment_root() {
        let res = segment(&mkv[47..100]);
        println!("{:?}", res);

        if let IResult::Done(i, _) = res {
            println!("consumed {} bytes after header", (&mkv[47..]).offset(i));
        }

        panic!();
    }

    #[test]
    fn mkv_segment_elements() {
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

    #[test]
    fn webm_segment_root() {
        let res = segment(&webm[40..100]);
        println!("{:?}", res);

        if let IResult::Done(i, _) = res {
            println!("consumed {} bytes after header", (&webm[40..]).offset(i));
        }

        panic!();
    }

    #[test]
    fn webm_segment_elements() {
        let mut index: usize = 48;

        loop {
            let res = segment_element(&webm[index..]);

            match res {
                IResult::Done(i, o) => {
                    let new_index = webm.offset(i);
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
                    let max_index = min(webm.len(), index + 200);
                    println!("[{}] {:#?}:\n{}",
                             index,
                             e,
                             (&webm[index..max_index]).to_hex(16));
                    break;
                }
            }
        }

        panic!();
    }
}
