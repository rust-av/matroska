#![allow(unused_assignments)]
use crate::ebml::{vid, vint, Error};
use nom::{
    number::streaming::{be_i16, be_u8},
    IResult,
};

#[derive(Debug, Clone, PartialEq)]
pub enum SegmentElement<'a> {
    SeekHead(SeekHead),
    Info(Info),
    Tracks(Tracks),
    Chapters(Chapters),
    Cluster(Cluster<'a>),
    Cues(Cues),
    Attachments(Attachments),
    Tags(Tags),
    Void(u64),
    Unknown(u64, Option<u64>),
}

// https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.3
named!(pub segment<&[u8], (u64, Option<u64>), Error>,
  do_parse!(
    id:   verify!(vid, |val:&u64| *val == 0x18538067) >>
    size: opt!(vint)   >>
    (id, size)
  )
);

#[macro_export]
macro_rules! sub_element(
  ($i:expr, $parser:ident) => ({
    do_parse!($i,
         size: vint
      >> crc: opt!(ebml_binary!(0xBF))
      >> element: flat_map!(take!((size - if crc.is_some() { 6 } else { 0 }) as usize), $parser)
      >> (element)
    )
  });
  ($i:expr, $submac:ident!( $($args:tt)* )) => ({
    do_parse!($i,
         size: vint
      >> crc: opt!(ebml_binary!(0xBF))
      >> element: flat_map!(take!((size - if crc.is_some() { 6 } else { 0 }) as usize), $submac!($($args)*))
      >> (element)
    )
  });
);

// Segment, the root element, has id 0x18538067
named!(pub segment_element<&[u8], SegmentElement, Error>,
  switch!(vid,
      0x114D9B74 => sub_element!(seek_head)
    | 0x1549A966 => sub_element!(info)
    | 0x1F43B675 => sub_element!(cluster)
    | 0x1043A770 => sub_element!(chapters)
    | 0x1254C367 => sub_element!(call!(ret_tags))
    | 0x1941A469 => sub_element!(call!(ret_attachments))
    | 0x1654AE6B => sub_element!(tracks)
    | 0x1C53BB6B => sub_element!(call!(ret_cues))
    | 0xEC       => do_parse!(size: vint >> take!(size as usize) >> (SegmentElement::Void(size)))
    | unknown    => do_parse!(
        size: opt!(vint) >>
              cond!(size.is_some(), take!( (size.unwrap() as usize) )) >>
              (SegmentElement::Unknown(unknown, size))
      )
  )
);

// hack to fix type inference issues
pub fn ret_tags(input: &[u8]) -> IResult<&[u8], SegmentElement, Error> {
    Ok((input, SegmentElement::Tags(Tags {})))
}

// hack to fix type inference issues
pub fn ret_attachments(input: &[u8]) -> IResult<&[u8], SegmentElement, Error> {
    Ok((input, SegmentElement::Attachments(Attachments {})))
}

// hack to fix type inference issues
pub fn ret_cues(input: &[u8]) -> IResult<&[u8], SegmentElement, Error> {
    Ok((input, SegmentElement::Cues(Cues {})))
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeekHead {
    pub positions: Vec<Seek>,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.4
named!(pub seek_head<&[u8], SegmentElement, Error>,
  do_parse!(
    positions: many1!(complete!(seek)) >>
    (SegmentElement::SeekHead(SeekHead {
      positions: positions,
    }))
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct Seek {
    pub id: Vec<u8>,
    pub position: u64,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.4
named!(pub seek<&[u8], Seek, Error>,
  ebml_master!(0x4DBB,
    do_parse!(
      t: permutation_opt!(
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

#[derive(Debug, Clone, PartialEq, Default)]
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
    pub duration: Option<f64>,     // FIXME should be float
    pub date_utc: Option<Vec<u8>>, //FIXME: should be date
    pub title: Option<String>,
    pub muxing_app: String,
    pub writing_app: String,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.8
named!(pub info<&[u8], SegmentElement, Error>,
  do_parse!(
    t: permutation_opt!(
      ebml_binary!(0x73A4)?, // SegmentUID
      ebml_str!(0x7384)?,    // SegmentFIlename FIXME SHOULD BE UTF-8 not str
      ebml_binary!(0x3CB923)?,         // PrevUID
      ebml_str!(0x3C83AB)?,            // PrevFilename FIXME SHOULD BE UTF-8 not str
      ebml_binary!(0x3EB923)?,         // NextUID
      ebml_str!(0x3E83BB)?,            // NextFilename FIXME SHOULD BE UTF-8 not str
      ebml_binary!(0x4444)?,           // SegmentFamily
      chapter_translate?,              //
      ebml_uint!(0x2AD7B1),            // TimecodeScale
      ebml_float!(0x4489)?,           // Duration: FIXME should be float
      ebml_binary!(0x4461)?,           // DateUTC FIXME: should be date
      ebml_str!(0x7BA9)?,              // Title FIXME SHOULD BE UTF-8 not str
      ebml_str!(0x4D80),               // MuxingApp FIXME SHOULD BE UTF-8 not str
      ebml_str!(0x5741)                // WritingApp FIXME SHOULD BE UTF-8 not str
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

#[derive(Debug, Clone, PartialEq)]
pub struct ChapterTranslate {}

// hack to fix type inference issues
pub fn ret_chapter_translate(input: &[u8]) -> IResult<&[u8], ChapterTranslate, Error> {
    Ok((input, ChapterTranslate {}))
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
named!(pub chapter_translate<&[u8], ChapterTranslate, Error>,
  //ebml_master!(0x6924, value!(ChapterTranslate{}))
  ebml_master!(0x6924, call!(ret_chapter_translate))
);

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.26
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster<'a> {
    pub timecode: u64,
    pub silent_tracks: Option<SilentTracks>,
    pub position: Option<u64>,
    pub prev_size: Option<u64>,
    pub simple_block: Vec<&'a [u8]>,
    pub block_group: Vec<BlockGroup<'a>>,
    pub encrypted_block: Option<&'a [u8]>,
}

named!(pub cluster<&[u8], SegmentElement, Error>,
  do_parse!(
    t: permutation_opt!(
      ebml_uint!(0xE7),
      silent_tracks?,
      ebml_uint!(0xA7)?,
      ebml_uint!(0xAB)?,
      ebml_binary_ref!(0xA3)+,
      block_group+,
      ebml_binary_ref!(0xAF)?
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

#[derive(Debug, Clone, PartialEq)]
pub struct SilentTracks {
    pub numbers: Vec<u64>,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
named!(pub silent_tracks<&[u8], SilentTracks, Error>,
  ebml_master!(0x5854, map!(many0!(ebml_uint!(0x58D7)), |v| SilentTracks { numbers: v }))
);

#[derive(Debug, Clone, PartialEq)]
pub struct BlockGroup<'a> {
    pub block: &'a [u8],
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
named!(pub block_group<&[u8], BlockGroup, Error>,
  ebml_master!(0x5854,
    do_parse!(
      t: permutation_opt!(
        ebml_binary_ref!(0xA1),
        ebml_binary!(0xA2)?,
        block_additions?,
        ebml_uint!(0x9B)?,
        ebml_uint!(0xFA),
        ebml_uint!(0xFB)?,
        ebml_int!(0xFD)?,
        ebml_binary!(0xA4)?,
        ebml_int!(0x75A2)?,
        slices?,
        reference_frame?

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

#[derive(Debug, Clone, PartialEq)]
pub struct BlockAdditions {}

// hack to fix type inference issues
pub fn ret_block_additions(input: &[u8]) -> IResult<&[u8], BlockAdditions, Error> {
    Ok((input, BlockAdditions {}))
}
//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
//TODO
named!(pub block_additions<&[u8], BlockAdditions, Error>,
  ebml_master!(0x75A1, call!(ret_block_additions))
);

#[derive(Debug, Clone, PartialEq)]
pub struct Slices {}

// hack to fix type inference issues
pub fn ret_slices(input: &[u8]) -> IResult<&[u8], Slices, Error> {
    Ok((input, Slices {}))
}
//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.46
//TODO
named!(pub slices<&[u8], Slices, Error>,
  ebml_master!(0x8E, call!(ret_slices))
);

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceFrame {}

// hack to fix type inference issues
pub fn ret_reference_frame(input: &[u8]) -> IResult<&[u8], ReferenceFrame, Error> {
    Ok((input, ReferenceFrame {}))
}
//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.53
//TODO
named!(pub reference_frame<&[u8], ReferenceFrame, Error>,
  ebml_master!(0xC8, call!(ret_reference_frame))
);

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub track_number: u64,
    pub timecode: i16,
    pub invisible: bool,
    pub lacing: Lacing,
}

named!(pub block<&[u8], Block, Error>,
  do_parse!(
       track_number: vint
    >> timecode:     be_i16
    >> flags:        map_opt!(be_u8, block_flags)
    >> (Block {
      track_number: track_number,
      timecode:     timecode,
      invisible:    flags.invisible,
      lacing:       flags.lacing,
    })
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct BlockFlags {
    pub keyframe: bool,
    pub invisible: bool,
    pub lacing: Lacing,
    pub discardable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleBlock {
    pub track_number: u64,
    pub timecode: i16,
    pub keyframe: bool,
    pub invisible: bool,
    pub lacing: Lacing,
    pub discardable: bool,
}

fn block_flags(data: u8) -> Option<BlockFlags> {
    let lacing_data = ((data << 6) >> 6) >> 5;
    let lacing = match lacing_data {
        0 => Lacing::None,
        1 => Lacing::Xiph,
        2 => Lacing::FixedSize,
        3 => Lacing::EBML,
        _ => return None,
    };

    Some(BlockFlags {
        keyframe: (data & 1) != 0,
        invisible: (data & (1 << 4)) != 0,
        lacing: lacing,
        discardable: (data & (1 << 7)) != 0,
    })
}

named!(pub simple_block<&[u8], SimpleBlock, Error>,
  do_parse!(
       track_number: vint
    >> timecode:     be_i16
    >> flags:        map_opt!(be_u8, block_flags)
    >> (SimpleBlock {
      track_number: track_number,
      timecode:     timecode,
      keyframe:     flags.keyframe,
      invisible:    flags.invisible,
      lacing:       flags.lacing,
      discardable:  flags.discardable,
    })
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleBlockFlags {
    pub keyframe: bool,
    pub invisible: bool,
    pub lacing: Lacing,
    pub discardable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lacing {
    None,
    Xiph,
    EBML,
    FixedSize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LacedData {
    pub frame_count: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tracks {
    pub tracks: Vec<TrackEntry>,
}

impl Tracks {
    pub fn lookup(&self, track_number: u64) -> Option<usize> {
        self.tracks
            .iter()
            .find(|t| t.track_number == track_number)
            .map(|t| t.stream_index)
    }
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
named!(pub tracks<&[u8], SegmentElement, Error>,
  map!(many1!(complete!(eat_void!(track_entry))), |v| SegmentElement::Tracks(Tracks { tracks: v }))
);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrackEntry {
    pub track_number: u64,
    pub track_uid: u64,
    pub track_type: u64,
    pub flag_enabled: Option<u64>, //FIXME: this flag is mandatory but does not appear in some files?
    pub flag_default: Option<u64>, //FIXME: this flag is mandatory but does not appear in some files?
    pub flag_forced: Option<u64>, //FIXME: this flag is mandatory but does not appear in some files?
    pub flag_lacing: Option<u64>, //FIXME: this flag is mandatory but does not appear in some files?
    pub min_cache: Option<u64>,   //FIXME: this flag is mandatory but does not appear in some files?
    pub max_cache: Option<u64>,
    pub default_duration: Option<u64>,
    pub default_decoded_field_duration: Option<u64>,
    pub track_timecode_scale: Option<f64>, //FIXME: this flag is mandatory but does not appear in some files?
    pub track_offset: Option<i64>,
    pub max_block_addition_id: Option<u64>, //FIXME: this flag is mandatory but does not appear in some files?
    pub name: Option<String>,
    pub language: Option<String>,
    pub language_ietf: Option<String>,
    pub codec_id: String,
    pub codec_private: Option<Vec<u8>>,
    pub codec_name: Option<String>,
    pub attachment_link: Option<u64>,
    pub codec_settings: Option<String>,
    pub codec_info_url: Option<String>,
    pub codec_download_url: Option<String>,
    pub codec_decode_all: Option<u64>, //FIXME: this flag is mandatory but does not appear in some files?
    pub track_overlay: Option<u64>,
    pub codec_delay: Option<u64>,
    pub seek_pre_roll: Option<u64>, //FIXME: this flag is mandatory but does not appear in some files?
    pub trick_track_uid: Option<u64>,
    pub trick_track_segment_uid: Option<Vec<u8>>,
    pub trick_track_flag: Option<u64>,
    pub trick_master_track_uid: Option<u64>,
    pub trick_master_track_segment_uid: Option<Vec<u8>>,
    pub video: Option<Video>,
    pub audio: Option<Audio>,
    pub track_translate: Vec<TrackTranslate>,
    pub track_operation: Option<TrackOperation>,
    pub content_encodings: Option<ContentEncodings>,
    /// The demuxer Stream index matching the Track
    pub stream_index: usize,
}

named!(pub track_entry<&[u8], TrackEntry, Error>,
  ebml_master!(0xAE,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0xD7),
        ebml_uint!(0x73C5),
        ebml_uint!(0x83),
        ebml_uint!(0xB9)?,
        ebml_uint!(0x88)?,
        ebml_uint!(0x55AA)?,
        ebml_uint!(0x9C)?,
        ebml_uint!(0x6DE7)?,
        ebml_uint!(0x6DF8)?,
        ebml_uint!(0x23E383)?,
        ebml_uint!(0x234E7A)?,
        ebml_float!(0x23314F)?,
        ebml_int!(0x537F)?,
        ebml_uint!(0x55EE)?,
        ebml_str!(0x536E)?,
        ebml_str!(0x22B59C)?,
        ebml_str!(0x22B59D)?,
        ebml_str!(0x86),
        ebml_binary!(0x63A2)?,
        ebml_str!(0x258688)?,
        ebml_uint!(0x7446)?,
        ebml_str!(0x3A9697)?,
        ebml_str!(0x3B4040)?,
        ebml_str!(0x26B240)?,
        ebml_uint!(0xAA)?,
        ebml_uint!(0x6FAB)?,
        ebml_uint!(0x56AA)?,
        ebml_uint!(0x56BB)?,
        track_translate+,
        video?,
        audio?,
        track_operation?,
        ebml_uint!(0xC0)?,
        ebml_binary!(0xC1)?,
        ebml_uint!(0xC6)?,
        ebml_uint!(0xC7)?,
        ebml_binary!(0xC4)?,
        content_encodings?
      ) >> (TrackEntry {
        track_number: t.0,
        track_uid: t.1,
        track_type: t.2,
        flag_enabled: t.3,
        flag_default: t.4,
        flag_forced: t.5,
        flag_lacing: t.6,
        min_cache: t.7,
        max_cache: t.8,
        default_duration: t.9,
        default_decoded_field_duration: t.10,
        track_timecode_scale: t.11,
        track_offset: t.12,
        max_block_addition_id: t.13,
        name: t.14,
        language: t.15,
        language_ietf: t.16,
        codec_id: t.17,
        codec_private: t.18,
        codec_name: t.19,
        attachment_link: t.20,
        codec_settings: t.21,
        codec_info_url: t.22,
        codec_download_url: t.23,
        codec_decode_all: t.24,
        track_overlay: t.25,
        codec_delay: t.26,
        seek_pre_roll: t.27,
        track_translate: t.28,
        video: t.29,
        audio: t.30,
        track_operation: t.31,
        trick_track_uid: t.32,
        trick_track_segment_uid: t.33,
        trick_track_flag: t.34,
        trick_master_track_uid: t.35,
        trick_master_track_segment_uid: t.36,
        content_encodings: t.37,
        stream_index: 0,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct TrackTranslate {
    pub edition_uid: Vec<u64>,
    pub codec: u64,
    pub track_id: u64,
}

named!(pub track_translate<&[u8], TrackTranslate, Error>,
  ebml_master!(0x6624,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0x66FC)+,
        ebml_uint!(0x66BF),
        ebml_uint!(0x66A5)
      ) >> (TrackTranslate {
        edition_uid: t.0,
        codec: t.1,
        track_id: t.2,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct TrackOperation {
    pub combine_planes: Option<TrackCombinePlanes>,
    pub join_blocks: Option<TrackJoinBlocks>,
}

named!(pub track_operation<&[u8], TrackOperation, Error>,
  ebml_master!(0xE2,
    do_parse!(
      t: permutation_opt!(
        track_combine_planes?,
        track_join_blocks?
      ) >> (TrackOperation {
        combine_planes: t.0,
        join_blocks:    t.1,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct TrackCombinePlanes {
    pub track_planes: Vec<TrackPlane>,
}

named!(pub track_combine_planes<&[u8], TrackCombinePlanes, Error>,
  ebml_master!(0xE3, map!(many1!(complete!(track_plane)), |v| TrackCombinePlanes { track_planes: v }))
);

#[derive(Debug, Clone, PartialEq)]
pub struct TrackPlane {
    pub uid: u64,
    pub plane_type: u64,
}

named!(pub track_plane<&[u8], TrackPlane, Error>,
  ebml_master!(0xE4,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0xE5),
        ebml_uint!(0xE6)
      ) >> (TrackPlane {
        uid:        t.0,
        plane_type: t.1,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct TrackJoinBlocks {
    pub uid: Vec<u64>,
}

named!(pub track_join_blocks<&[u8], TrackJoinBlocks, Error>,
  ebml_master!(0xE9, map!(many1!(complete!(ebml_uint!(0xED))), |v| TrackJoinBlocks { uid: v }))
);

#[derive(Debug, Clone, PartialEq)]
pub struct ContentEncodings {
    pub content_encoding: Vec<ContentEncoding>,
}

named!(pub content_encodings<&[u8], ContentEncodings, Error>,
  ebml_master!(0x6D80, map!(many1!(complete!(content_encoding)), |v| ContentEncodings { content_encoding: v }))
);

#[derive(Debug, Clone, PartialEq)]
pub struct ContentEncoding {
    order: u64,
    scope: u64,
    encoding_type: u64,
    compression: Option<ContentCompression>,
    encryption: Option<ContentEncryption>,
}

named!(pub content_encoding<&[u8], ContentEncoding, Error>,
  ebml_master!(0x6240,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0x5031),
        ebml_uint!(0x5032),
        ebml_uint!(0x5033),
        content_compression?,
        content_encryption?
      ) >> (ContentEncoding {
        order:         t.0,
        scope:         t.1,
        encoding_type: t.2,
        compression:   t.3,
        encryption:    t.4
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct ContentCompression {
    algo: u64,
    settings: Option<u64>,
}

named!(pub content_compression<&[u8], ContentCompression, Error>,
  ebml_master!(0x5034,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0x4254),
        ebml_uint!(0x4255)?
      ) >> (ContentCompression {
        algo:     t.0,
        settings: t.1,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct ContentEncryption {
    enc_algo: Option<u64>,
    enc_key_id: Option<Vec<u8>>,
    signature: Option<Vec<u8>>,
    sig_key_id: Option<Vec<u8>>,
    sig_algo: Option<u64>,
    sig_hash_algo: Option<u64>,
}

named!(pub content_encryption<&[u8], ContentEncryption, Error>,
  ebml_master!(0x5035,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0x47E1)?,
        ebml_binary!(0x47E2)?,
        ebml_binary!(0x47E3)?,
        ebml_binary!(0x47E4)?,
        ebml_uint!(0x47E5)?,
        ebml_uint!(0x47E6)?
      ) >> (ContentEncryption {
        enc_algo:      t.0,
        enc_key_id:    t.1,
        signature:     t.2,
        sig_key_id:    t.3,
        sig_algo:      t.4,
        sig_hash_algo: t.5,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Audio {
    pub sampling_frequency: f64,
    pub output_sampling_frequency: Option<f64>,
    pub channels: u64,
    pub channel_positions: Option<Vec<u8>>,
    pub bit_depth: Option<u64>,
}

named!(pub audio<&[u8], Audio, Error>,
  ebml_master!(0xE1,
    do_parse!(
      t: permutation_opt!(
        ebml_float!(0xB5),
        ebml_float!(0x78B5)?,
        ebml_uint!(0x9F),
        ebml_binary!(0x7D7B)?,
        ebml_uint!(0x6264)?
      ) >> (Audio {
        sampling_frequency: t.0,
        output_sampling_frequency: t.1,
        channels: t.2,
        channel_positions: t.3,
        bit_depth: t.4,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Video {
    pub flag_interlaced: Option<u64>,
    pub field_order: Option<u64>,
    pub stereo_mode: Option<u64>,
    pub alpha_mode: Option<u64>,
    pub old_stereo_mode: Option<u64>,
    pub pixel_width: u64,
    pub pixel_height: u64,
    pub pixel_crop_bottom: Option<u64>,
    pub pixel_crop_top: Option<u64>,
    pub pixel_crop_left: Option<u64>,
    pub pixel_crop_right: Option<u64>,
    pub display_width: Option<u64>,
    pub display_height: Option<u64>,
    pub display_unit: Option<u64>,
    pub aspect_ratio_type: Option<u64>,
    pub colour_space: Option<Vec<u8>>,
    pub gamma_value: Option<f64>,
    pub frame_rate: Option<f64>,
    pub colour: Option<Colour>,
    pub projection: Option<Projection>,
}

named!(pub video<&[u8], Video, Error>,
  ebml_master!(0xE0,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0x9A)?,
        ebml_uint!(0x9D)?,
        ebml_uint!(0x53B8)?,
        ebml_uint!(0x53C0)?,
        ebml_uint!(0x53B9)?,
        ebml_uint!(0xB0),
        ebml_uint!(0xBA),
        ebml_uint!(0x54AA)?,
        ebml_uint!(0x54BB)?,
        ebml_uint!(0x54CC)?,
        ebml_uint!(0x54DD)?,
        ebml_uint!(0x54B0)?,
        ebml_uint!(0x54BA)?,
        ebml_uint!(0x54B2)?,
        ebml_uint!(0x54B3)?,
        ebml_binary!(0x2EB524)?,
        ebml_float!(0x2FB523)?,
        ebml_float!(0x2383E3)?,
        colour?,
        projection?
      ) >> (Video {
        flag_interlaced: t.0,
        field_order: t.1,
        stereo_mode: t.2,
        alpha_mode: t.3,
        old_stereo_mode: t.4,
        pixel_width: t.5,
        pixel_height: t.6,
        pixel_crop_bottom: t.7,
        pixel_crop_top: t.8,
        pixel_crop_left: t.9,
        pixel_crop_right: t.10,
        display_width: t.11,
        display_height: t.12,
        display_unit: t.13,
        aspect_ratio_type: t.14,
        colour_space: t.15,
        gamma_value: t.16,
        frame_rate: t.17,
        colour: t.18,
        projection: t.19,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Colour {
    pub matrix_coefficients: Option<u64>,
    pub bits_per_channel: Option<u64>,
    pub chroma_subsampling_horz: Option<u64>,
    pub chroma_subsampling_vert: Option<u64>,
    pub cb_subsampling_horz: Option<u64>,
    pub cb_subsampling_vert: Option<u64>,
    pub chroma_siting_horz: Option<u64>,
    pub chroma_siting_vert: Option<u64>,
    pub range: Option<u64>,
    pub transfer_characteristics: Option<u64>,
    pub primaries: Option<u64>,
    pub max_cll: Option<u64>,
    pub max_fall: Option<u64>,
    pub mastering_metadata: Option<MasteringMetadata>,
}

named!(pub colour<&[u8], Colour, Error>,
  ebml_master!(0x55B0,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0x55B1)?,
        ebml_uint!(0x55B2)?,
        ebml_uint!(0x55B3)?,
        ebml_uint!(0x55B4)?,
        ebml_uint!(0x55B5)?,
        ebml_uint!(0x55B6)?,
        ebml_uint!(0x55B7)?,
        ebml_uint!(0x55B8)?,
        ebml_uint!(0x55B9)?,
        ebml_uint!(0x55BA)?,
        ebml_uint!(0x55BB)?,
        ebml_uint!(0x55BC)?,
        ebml_uint!(0x55BD)?,
        mastering_metadata?
      ) >> (Colour {
        matrix_coefficients: t.0,
        bits_per_channel: t.1,
        chroma_subsampling_horz: t.2,
        chroma_subsampling_vert: t.3,
        cb_subsampling_horz: t.4,
        cb_subsampling_vert: t.5,
        chroma_siting_horz: t.6,
        chroma_siting_vert: t.7,
        range: t.8,
        transfer_characteristics: t.9,
        primaries: t.10,
        max_cll: t.11,
        max_fall: t.12,
        mastering_metadata: t.13,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct MasteringMetadata {
    pub primary_r_chromaticity_x: Option<f64>,
    pub primary_r_chromaticity_y: Option<f64>,
    pub primary_g_chromaticity_x: Option<f64>,
    pub primary_g_chromaticity_y: Option<f64>,
    pub primary_b_chromaticity_x: Option<f64>,
    pub primary_b_chromaticity_y: Option<f64>,
    pub white_point_chromaticity_x: Option<f64>,
    pub white_point_chromaticity_y: Option<f64>,
    pub luminance_max: Option<f64>,
    pub luminance_min: Option<f64>,
}

named!(pub mastering_metadata<&[u8], MasteringMetadata, Error>,
  ebml_master!(0x55D0,
    do_parse!(
      t: permutation_opt!(
        ebml_float!(0x55D1)?,
        ebml_float!(0x55D2)?,
        ebml_float!(0x55D3)?,
        ebml_float!(0x55D4)?,
        ebml_float!(0x55D5)?,
        ebml_float!(0x55D6)?,
        ebml_float!(0x55D7)?,
        ebml_float!(0x55D8)?,
        ebml_float!(0x55D9)?,
        ebml_float!(0x55DA)?
      ) >> (MasteringMetadata {
        primary_r_chromaticity_x: t.0,
        primary_r_chromaticity_y: t.1,
        primary_g_chromaticity_x: t.2,
        primary_g_chromaticity_y: t.3,
        primary_b_chromaticity_x: t.4,
        primary_b_chromaticity_y: t.5,
        white_point_chromaticity_x: t.6,
        white_point_chromaticity_y: t.7,
        luminance_max: t.8,
        luminance_min: t.9,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    pub projection_type: u64,
    pub projection_private: Option<Vec<u8>>,
    pub projection_pose_yaw: f64,
    pub projection_pose_pitch: f64,
    pub projection_pose_roll: f64,
}

named!(pub projection<&[u8], Projection, Error>,
  ebml_master!(0x7670,
    do_parse!(
      t: permutation_opt!(
        ebml_uint!(0x7671),
        ebml_binary!(0x7672)?,
        ebml_float!(0x7673),
        ebml_float!(0x7674),
        ebml_float!(0x7675)
      ) >> (Projection {
        projection_type: t.0,
        projection_private: t.1,
        projection_pose_yaw: t.2,
        projection_pose_pitch: t.3,
        projection_pose_roll: t.4,
      })
    )
  )
);

#[derive(Debug, Clone, PartialEq)]
pub struct Chapters {}

// hack to fix type inference issues
pub fn ret_chapters(input: &[u8]) -> IResult<&[u8], SegmentElement, Error> {
    Ok((input, SegmentElement::Chapters(Chapters {})))
}
//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.199
//TODO
named!(pub chapters<&[u8], SegmentElement, Error>,
  //EditionEntry
  ebml_master!(0x45B9, call!(ret_chapters))
);

#[derive(Debug, Clone, PartialEq)]
pub struct Cues {}

#[derive(Debug, Clone, PartialEq)]
pub struct Attachments {}

#[derive(Debug, Clone, PartialEq)]
pub struct Tags {}

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use super::*;
    use log::debug;
    use nom::{HexDisplay, Offset};
    use std::cmp::min;

    const mkv: &'static [u8] = include_bytes!("../assets/single_stream.mkv");
    const webm: &'static [u8] = include_bytes!("../assets/big-buck-bunny_trailer.webm");

    #[test]
    fn mkv_segment_root() {
        let res = segment(&mkv[47..100]);
        debug!("{:?}", res);

        if let Ok((i, _)) = res {
            debug!("consumed {} bytes after header", (&mkv[47..]).offset(i));
        } else {
            panic!("res: {:?}", res);
        }
    }

    #[test]
    fn mkv_segment_elements() {
        let mut index: usize = 59;

        loop {
            let res = segment_element(&mkv[index..]);

            match res {
                Ok((i, o)) => {
                    let new_index = mkv.offset(i);
                    match o {
                        SegmentElement::Unknown(id, size) => {
                            debug!(
                                "[{} -> {}] Unknown {{ id: 0x{:x}, size: {:?} }}",
                                index, new_index, id, size
                            );
                        }
                        o => {
                            debug!("[{} -> {}] {:#?}", index, new_index, o);
                        }
                    };

                    index = new_index as usize;
                }
                e => {
                    let max_index = min(mkv.len(), index + 200);
                    debug!(
                        "[{}] {:#?}:\n{}",
                        index,
                        e,
                        (&mkv[index..max_index]).to_hex(16)
                    );
                    break;
                }
            }
        }

        //panic!();
    }

    #[test]
    fn webm_segment_root() {
        let res = segment(&webm[40..100]);
        debug!("{:?}", res);

        if let Ok((i, _)) = res {
            debug!("consumed {} bytes after header", (&webm[40..]).offset(i));
        } else {
            panic!("res: {:?}", res);
        }
    }

    #[test]
    fn webm_segment_elements() {
        let mut index: usize = 48;

        loop {
            let res = segment_element(&webm[index..]);

            match res {
                Ok((i, o)) => {
                    let new_index = webm.offset(i);
                    match o {
                        SegmentElement::Unknown(id, size) => {
                            debug!(
                                "[{} -> {}] Unknown {{ id: 0x{:x}, size: {:?} }}",
                                index, new_index, id, size
                            );
                        }
                        o => {
                            debug!("[{} -> {}] {:#?}", index, new_index, o);
                        }
                    };

                    index = new_index as usize;
                }
                e => {
                    let max_index = min(webm.len(), index + 200);
                    debug!(
                        "[{}] {:#?}:\n{}",
                        index,
                        e,
                        (&webm[index..max_index]).to_hex(16)
                    );
                    break;
                }
            }
        }
    }
}
