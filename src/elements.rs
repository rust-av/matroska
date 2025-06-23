use nom::{
    bytes::streaming::take,
    combinator::{map, map_opt, opt},
    number::streaming::{be_i16, be_u8},
    sequence::{pair, tuple},
};

pub use uuid::Uuid;

use crate::ebml::{check_id, checksum, crc, elem_size, vid, vint, EbmlParsable, EbmlResult, Error};
use crate::ebml::{macros::impl_ebml_master, Date};
use crate::elements;

#[derive(Debug, Clone, PartialEq)]
pub enum SegmentElement<'a> {
    SeekHead(SeekHead),
    Info(Info),
    Tracks(Tracks),
    // Chapters(Chapters),
    Cluster(Cluster<'a>),
    // Cues(Cues),
    // Attachments(Attachments),
    // Tags(Tags),
    Void(usize),
    Unknown(u32, Option<usize>),
}

// https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.3
pub fn segment(input: &[u8]) -> EbmlResult<(u32, Option<u64>)> {
    pair(check_id(0x18538067), opt(vint))(input)
}

pub(crate) fn sub_element<'a, O: EbmlParsable<'a>>(input: &'a [u8]) -> EbmlResult<'a, O> {
    let (i, (mut size, crc)) = pair(elem_size, crc)(input)?;

    if crc.is_some() {
        size -= 6;
    }

    let (i, data) = checksum(crc, take(size))(i)?;

    match O::try_parse(data) {
        Ok(o) => Ok((i, o)),
        Err(kind) => Err(nom::Err::Error(Error { id: 0, kind })),
    }
}

// Segment, the root element, has id 0x18538067
pub fn segment_element(input: &[u8]) -> EbmlResult<SegmentElement> {
    use SegmentElement::*;

    vid(input).and_then(|(i, id)| {
        match id {
            0x114D9B74 => sub_element::<elements::SeekHead>(i).map(|(i, sh)| (i, SeekHead(sh))),
            0x1549A966 => sub_element::<elements::Info>(i).map(|(i, info)| (i, Info(info))),
            0x1F43B675 => sub_element::<elements::Cluster>(i).map(|(i, cl)| (i, Cluster(cl))),
            // 0x1043A770 => sub_element(|i| Ok((i, SegmentElement::Chapters(Chapters {}))))(i),
            // 0x1254C367 => sub_element(|i| Ok((i, SegmentElement::Tags(Tags {}))))(i),
            // 0x1941A469 => sub_element(|i| Ok((i, SegmentElement::Attachments(Attachments {}))))(i),
            0x1654AE6B => sub_element::<elements::Tracks>(i).map(|(i, tr)| (i, Tracks(tr))),
            // 0x1C53BB6B => sub_element(|i| Ok((i, SegmentElement::Cues(Cues {}))))(i),
            0xEC => {
                let (i, size) = elem_size(i)?;
                take(size)(i).map(|(i, _)| (i, Void(size)))
            }
            id => {
                let (i, size) = opt(elem_size)(i)?;
                match size {
                    Some(sz) => {
                        take(sz)(i).map(|(i, _)| (i, SegmentElement::Unknown(id, Some(sz))))
                    }
                    None => Ok((i, SegmentElement::Unknown(id, None))),
                }
            }
        }
    })
}

impl_ebml_master! {
    // Element ID 0x114D9B74
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct SeekHead {
        [0x4DBB] positions: (Vec<Seek>) [1..],
    }
}

impl_ebml_master! {
    // Element ID 0x4DBB
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Seek {
        [0x53AB] id: ([u8; 4]),
        [0x53AC] position: (u64),
    }
}

// FIXME: Strings should be UTF-8, not ASCII
impl_ebml_master! {
    // Element ID 0x1549A966
    #[derive(Debug, Default, Clone, PartialEq)]
    struct Info {
        [0x73A4] segment_uid: (Option<Uuid>),
        [0x7384] segment_filename: (Option<String>),
        [0x3CB923] prev_uid: (Option<Uuid>),
        [0x3C83AB] prev_filename: (Option<String>),
        [0x3EB923] next_uid: (Option<Uuid>),
        [0x3E83BB] next_filename: (Option<String>),
        [0x4444] segment_family: (Option<Uuid>),
        // [0x6924] chapter_translate: (Option<ChapterTranslate>),
        [0x2AD7B1] timestamp_scale: (u64) = 1000000,
        [0x4489] duration: (Option<f64>),     // FIXME: should be float
        [0x4461] date_utc: (Option<Date>),
        [0x7BA9] title: (Option<String>),
        [0x4D80] muxing_app: (String),
        [0x5741] writing_app: (String),
    }
}

impl_ebml_master! {
    // Element ID 0x1F43B675
    #[derive(Debug, Clone, PartialEq)]
    struct Cluster<'a> {
        [0xE7] timestamp: (u64),
        [0xA7] position: (Option<u64>),
        [0xAB] prev_size: (Option<u64>),
        [0xA3] simple_block: (Vec<&'a [u8]>) [0..],
        [0xA0] block_group: (Vec<BlockGroup<'a>>) [0..],
    }
}

impl_ebml_master! {
    // Element ID 0xA0
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct BlockGroup<'a> {
        [0xA1] block: (&'a [u8]),
        // [0x75A1] block_additions: (Option<BlockAdditions>),
        [0x9B] block_duration: (Option<u64>),
        [0xFA] reference_priority: (u64) = 0,
        [0xFB] reference_block: (Option<u64>),
        [0xA4] codec_state: (Option<Vec<u8>>),
        [0x75A2] discard_padding: (Option<i64>),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub track_number: u64,
    pub timestamp: i16,
    pub invisible: bool,
    pub lacing: Lacing,
}

pub fn block(input: &[u8]) -> EbmlResult<Block> {
    map(
        tuple((vint, be_i16, map_opt(be_u8, block_flags))),
        |(track_number, timestamp, flags)| Block {
            track_number,
            timestamp,
            invisible: flags.invisible,
            lacing: flags.lacing,
        },
    )(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockFlags {
    pub keyframe: bool,
    pub invisible: bool,
    pub lacing: Lacing,
    pub discardable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleBlock {
    pub track_number: u64,
    pub timestamp: i16,
    pub keyframe: bool,
    pub invisible: bool,
    pub lacing: Lacing,
    pub discardable: bool,
}

fn block_flags(data: u8) -> Option<BlockFlags> {
    let lacing_data = ((data << 5) >> 5) >> 1;
    let lacing = match lacing_data {
        0 => Lacing::None,
        1 => Lacing::Xiph,
        2 => Lacing::FixedSize,
        3 => Lacing::EBML,
        _ => return None,
    };

    Some(BlockFlags {
        keyframe: (data & (1 << 7)) != 0,
        invisible: (data & (1 << 3)) != 0,
        lacing,
        discardable: (data & 1) != 0,
    })
}

pub fn simple_block(input: &[u8]) -> EbmlResult<SimpleBlock> {
    map(
        tuple((vint, be_i16, map_opt(be_u8, block_flags))),
        |(track_number, timestamp, flags)| SimpleBlock {
            track_number,
            timestamp,
            keyframe: flags.keyframe,
            invisible: flags.invisible,
            lacing: flags.lacing,
            discardable: flags.discardable,
        },
    )(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleBlockFlags {
    pub keyframe: bool,
    pub invisible: bool,
    pub lacing: Lacing,
    pub discardable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lacing {
    None,
    Xiph,
    EBML,
    FixedSize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LacedData {
    pub frame_count: u8,
}

impl_ebml_master! {
    // Element ID 0x1654AE6B
    #[derive(Debug, Clone, PartialEq)]
    struct Tracks {
        [0xAE] tracks: (Vec<TrackEntry>) [1..],
    }
}

impl Tracks {
    pub fn lookup(&self, track_number: u64) -> Option<usize> {
        self.tracks
            .iter()
            .find(|t| t.track_number == track_number)
            .map(|t| t.stream_index as usize)
    }
}

pub(crate) enum TrackType {
    Video,
    Audio,
    Other,
}

impl From<u64> for TrackType {
    fn from(val: u64) -> Self {
        match val {
            0x1 => Self::Video,
            0x2 => Self::Audio,
            _ => Self::Other,
        }
    }
}

impl From<TrackType> for u64 {
    fn from(val: TrackType) -> Self {
        match val {
            TrackType::Video => 0x1,
            TrackType::Audio => 0x2,
            TrackType::Other => 0,
        }
    }
}

impl_ebml_master! {
    // Element ID 0xAE
    #[derive(Debug, Clone, PartialEq, Default)]
    struct TrackEntry {
        [0xD7] track_number: (u64),
        [0x73C5] track_uid: (u64),
        [0x83] track_type: (u64),
        [0xB9] flag_enabled: (u64) = 1,
        [0x88] flag_default: (u64) = 1,
        [0x55AA] flag_forced: (u64) = 0,
        [0x9C] flag_lacing: (u64) = 1,
        [0x23E383] default_duration: (Option<u64>),
        [0x234E7A] default_decoded_field_duration: (Option<u64>),
        // FIXME: reimplement float_or handling
        [0x23314F] track_timestamp_scale: (f64) = 1.0,
        [0x55EE] max_block_addition_id: (u64) = 0,
        [0x536E] name: (Option<String>),
        [0x22B59C] language: (String) = String::from("eng"),
        [0x22B59D] language_ietf: (Option<String>),
        [0x86] codec_id: (String),
        [0x63A2] codec_private: (Option<Vec<u8>>),
        [0x258688] codec_name: (Option<String>),
        [0x7446] attachment_link: (Option<u64>),
        [0x56AA] codec_delay: (u64) = 0,
        [0x56BB] seek_pre_roll: (u64) = 0,
        [0xE0] video: (Option<Video>),
        [0xE1] audio: (Option<Audio>),
        [0x6624] track_translate: (Vec<TrackTranslate>) [0..],
        [0xE2] track_operation: (Option<TrackOperation>),
        [0x6D80] content_encodings: (Option<ContentEncodings>),
        // The demuxer Stream index matching the Track
        // ID 0xFFFFFFFF is a workaround because this is not data
        // from the Matroska format, but something else.
        [0xFFFFFFFF] stream_index: (u64) = 0, // FIXME: Move somewhere else?
    }
}

impl_ebml_master! {
    // Element ID 0xC7
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TrackTranslate {
        [0x66FC] edition_uid: (Vec<u64>) [0..],
        [0x66BF] codec: (u64),
        [0x66A5] track_id: (u64),
    }
}

impl_ebml_master! {
    // Element ID 0xC4
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TrackOperation {
        [0xE3] combine_planes: (Option<TrackCombinePlanes>),
        [0xE9] join_blocks: (Option<TrackJoinBlocks>),
    }
}

impl_ebml_master! {
    // Element ID 0xE3
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TrackCombinePlanes {
        [0xE4] track_planes: (Vec<TrackPlane>) [1..],
    }
}

impl_ebml_master! {
    // Element ID 0xE4
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TrackPlane {
        [0xE5] uid: (u64),
        [0xE6] plane_type: (u64),
    }
}

impl_ebml_master! {
    // Element ID 0xE9
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TrackJoinBlocks {
        [0xED] uid: (Vec<u64>) [1..],
    }
}

impl_ebml_master! {
    // Element ID 0x6D80
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ContentEncodings {
        [0x6240] content_encoding: (Vec<ContentEncoding>) [1..],
    }
}

impl_ebml_master! {
    // Element ID 0x6240
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ContentEncoding {
        [0x5031] order: (u64) = 0,
        [0x5032] scope: (u64) = 1,
        [0x5033] encoding_type: (u64) = 0,
        [0x5034] compression: (Option<ContentCompression>),
        [0x5035] encryption: (Option<ContentEncryption>),
    }
}

impl_ebml_master! {
    // Element ID 0x5034
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ContentCompression {
        [0x4254] algo: (u64) = 0,
        [0x4255] settings: (Option<u64>),
    }

}

impl_ebml_master! {
    // Element ID 0x5035
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ContentEncryption {
        [0x47E1] enc_algo: (u64) = 0,
        [0x47E2] enc_key_id: (Option<Vec<u8>>),
    }
}

impl_ebml_master! {
    // Element ID 0xC6
    #[derive(Debug, Clone, PartialEq, Default)]
    struct Audio {
        // FIXME: reimplement float_or handling
        [0xB5] sampling_frequency: (f64) = 5360.0,
        [0x7885] output_sampling_frequency: (Option<f64>),
        [0x9F] channels: (u64),
        [0x6264] bit_depth: (Option<u64>),
    }
}

impl_ebml_master! {
    // Element ID 0xC1
    #[derive(Debug, Clone, PartialEq, Default)]
    struct Video {
        [0x9A] flag_interlaced: (u64) = 0,
        [0x9D] field_order: (u64) = 2,
        [0x53B8] stereo_mode: (u64) = 0,
        [0x53C0] alpha_mode: (u64) = 0,
        [0x53B9] old_stereo_mode: (Option<u64>),
        [0xB0] pixel_width: (u64),
        [0xBA] pixel_height: (u64),
        [0x54AA] pixel_crop_bottom: (u64) = 0,
        [0x54BB] pixel_crop_top: (u64) = 0,
        [0x54CC] pixel_crop_left: (u64) = 0,
        [0x54DD] pixel_crop_right: (u64) = 0,
        [0x54B0] display_width: (Option<u64>),
        [0x54BA] display_height: (Option<u64>),
        [0x54B2] display_unit: (u64) = 0,
        [0x2EB524] colour_space: (Option<Vec<u8>>),
        [0x55B0] colour: (Option<Colour>),
        [0x55D0] projection: (Option<Projection>),
    }
}

impl_ebml_master! {
    // Element ID 0x55B0
    #[derive(Debug, Clone, PartialEq, Default)]
    struct Colour {
        [0x55B1] matrix_coefficients: (u64) = 2,
        [0x55B2] bits_per_channel: (u64) = 0,
        [0x55B3] chroma_subsampling_horz: (Option<u64>),
        [0x55B4] chroma_subsampling_vert: (Option<u64>),
        [0x55B5] cb_subsampling_horz: (Option<u64>),
        [0x55B6] cb_subsampling_vert: (Option<u64>),
        [0x55B7] chroma_siting_horz: (u64) = 0,
        [0x55B8] chroma_siting_vert: (u64) = 0,
        [0x55B9] range: (u64) = 0,
        [0x55BA] transfer_characteristics: (u64) = 2,
        [0x55BB] primaries: (u64) = 2,
        [0x55BC] max_cll: (Option<u64>),
        [0x55BD] max_fall: (Option<u64>),
        [0x55D0] mastering_metadata: (Option<MasteringMetadata>),
    }

}

impl_ebml_master! {
    // Element ID 0x55D0
    #[derive(Debug, Clone, PartialEq)]
    struct MasteringMetadata {
        [0x55D1] primary_r_chromaticity_x: (Option<f64>),
        [0x55D2] primary_r_chromaticity_y: (Option<f64>),
        [0x55D3] primary_g_chromaticity_x: (Option<f64>),
        [0x55D4] primary_g_chromaticity_y: (Option<f64>),
        [0x55D5] primary_b_chromaticity_x: (Option<f64>),
        [0x55D6] primary_b_chromaticity_y: (Option<f64>),
        [0x55D7] white_point_chromaticity_x: (Option<f64>),
        [0x55D8] white_point_chromaticity_y: (Option<f64>),
        [0x55D9] luminance_max: (Option<f64>),
        [0x55DA] luminance_min: (Option<f64>),
    }
}

impl_ebml_master! {
    // Element ID 0x7670
    #[derive(Debug, Clone, PartialEq)]
    struct Projection {
        [0x7671] projection_type: (u64) = 0,
        [0x7672] projection_private: (Option<Vec<u8>>),
        // FIXME: reimplement float_or handling
        [0x7673] projection_pose_yaw: (f64) = 0.0,
        [0x7674] projection_pose_pitch: (f64) = 0.0,
        [0x7675] projection_pose_roll: (f64) = 0.0,
    }
}

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use std::cmp::min;

    use nom::{HexDisplay, Offset};

    use super::*;

    const mkv: &[u8] = include_bytes!("../assets/single_stream.mkv");
    const webm: &[u8] = include_bytes!("../assets/big-buck-bunny_trailer.webm");

    #[test]
    fn block_flags() {
        let test = [
            (
                0b1000_1011,
                Some(BlockFlags {
                    keyframe: true,
                    invisible: true,
                    lacing: Lacing::Xiph,
                    discardable: true,
                }),
            ),
            (
                0b0000_0000,
                Some(BlockFlags {
                    keyframe: false,
                    invisible: false,
                    lacing: Lacing::None,
                    discardable: false,
                }),
            ),
            (
                0b0000_0110,
                Some(BlockFlags {
                    keyframe: false,
                    invisible: false,
                    lacing: Lacing::EBML,
                    discardable: false,
                }),
            ),
            (
                0b0000_0101,
                Some(BlockFlags {
                    keyframe: false,
                    invisible: false,
                    lacing: Lacing::FixedSize,
                    discardable: true,
                }),
            ),
        ];

        for (data, flags) in test {
            assert_eq!(super::block_flags(data), flags);
        }
    }

    #[test]
    fn mkv_segment_root() {
        let res = segment(&mkv[47..100]);
        println!("{res:?}");

        if let Ok((i, _)) = res {
            println!("consumed {} bytes after header", (mkv[47..]).offset(i));
        } else {
            panic!("res: {res:?}");
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
                            println!(
                                "[{index} -> {new_index}] Unknown {{ id: 0x{id:x}, size: {size:?} }}",
                            );
                        }
                        o => {
                            println!("[{index} -> {new_index}] {o:#?}");
                        }
                    };

                    index = new_index;
                }
                e => {
                    let max_index = min(mkv.len(), index + 200);
                    println!("[{index}] {e:#?}:\n{}", (mkv[index..max_index]).to_hex(16));
                    break;
                }
            }
        }
    }

    #[test]
    fn webm_segment_root() {
        let res = segment(&webm[40..100]);
        println!("{res:?}");

        if let Ok((i, _)) = res {
            println!("consumed {} bytes after header", (webm[40..]).offset(i));
        } else {
            panic!("res: {res:?}");
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
                            println!(
                                "[{index} -> {new_index}] Unknown {{ id: 0x{id:x}, size: {size:?} }}"
                            );
                        }
                        o => {
                            println!("[{index} -> {new_index}] {o:#?}");
                        }
                    };

                    index = new_index;
                }
                e => {
                    let max_index = min(webm.len(), index + 200);
                    println!("[{index}] {e:#?}:\n{}", (webm[index..max_index]).to_hex(16));
                    break;
                }
            }
        }
    }
}
