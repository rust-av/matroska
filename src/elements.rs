use nom::{
    bytes::streaming::take,
    combinator::{complete, cond, map, map_opt, opt, verify},
    multi::{many0, many1},
    number::streaming::{be_i16, be_u8},
    sequence::{pair, tuple},
};

pub use uuid::Uuid;

use crate::ebml::{
    binary, binary_ref, checksum, crc, elem_size, float, int, master, skip_void, str, uint, uuid,
    value_error, vid, vint, EbmlResult,
};
use crate::permutation::matroska_permutation;

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
    Void(usize),
    Unknown(u32, Option<usize>),
}

// https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.3
pub fn segment(input: &[u8]) -> EbmlResult<(u32, Option<u64>)> {
    pair(verify(vid, |val| *val == 0x18538067), opt(vint))(input)
}

pub fn sub_element<'a, O1, G>(second: G) -> impl Fn(&'a [u8]) -> EbmlResult<'a, O1>
where
    G: Fn(&'a [u8]) -> EbmlResult<'a, O1> + Copy,
{
    move |input| {
        pair(elem_size, crc)(input).and_then(|(i, (size, crc))| {
            let size = if crc.is_some() { size - 6 } else { size };
            checksum(crc, take(size))(i).and_then(|(i, data)| second(data).map(|(_, val)| (i, val)))
        })
    }
}

// Segment, the root element, has id 0x18538067
pub fn segment_element(input: &[u8]) -> EbmlResult<SegmentElement> {
    vid(input).and_then(|(i, id)| match id {
        0x114D9B74 => sub_element(seek_head)(i),
        0x1549A966 => sub_element(info)(i),
        0x1F43B675 => sub_element(cluster)(i),
        0x1043A770 => sub_element(chapters)(i),
        0x1254C367 => sub_element(|i| Ok((i, SegmentElement::Tags(Tags {}))))(i),
        0x1941A469 => sub_element(|i| Ok((i, SegmentElement::Attachments(Attachments {}))))(i),
        0x1654AE6B => sub_element(tracks)(i),
        0x1C53BB6B => sub_element(|i| Ok((i, SegmentElement::Cues(Cues {}))))(i),
        0xEC => {
            elem_size(i).and_then(|(i, size)| map(take(size), |_| SegmentElement::Void(size))(i))
        }
        id => opt(elem_size)(i).and_then(|(i, size)| {
            map(cond(size.is_some(), take(size.unwrap())), |_| {
                SegmentElement::Unknown(id, size)
            })(i)
        }),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeekHead {
    pub positions: Vec<Seek>,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.4
pub fn seek_head(input: &[u8]) -> EbmlResult<SegmentElement> {
    map(many1(complete(seek)), |positions| {
        SegmentElement::SeekHead(SeekHead { positions })
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seek {
    pub id: Vec<u8>,
    pub position: u64,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.4
pub fn seek(input: &[u8]) -> EbmlResult<Seek> {
    master(0x4DBB, |inp| {
        matroska_permutation((
            binary(0x53AB), // SeekID
            uint(0x53AC),   // SeekPosition
        ))(inp)
        .and_then(|(i, t)| {
            Ok((
                i,
                Seek {
                    id: value_error(0x53AB, t.0)?,
                    position: value_error(0x53AC, t.1)?,
                },
            ))
        })
    })(input)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Info {
    pub segment_uid: Option<Uuid>,
    pub segment_filename: Option<String>,
    pub prev_uid: Option<Uuid>,
    pub prev_filename: Option<String>,
    pub next_uid: Option<Uuid>,
    pub next_filename: Option<String>,
    pub segment_family: Option<Uuid>,
    pub chapter_translate: Option<ChapterTranslate>,
    pub timecode_scale: u64,
    pub duration: Option<f64>,     // FIXME should be float
    pub date_utc: Option<Vec<u8>>, //FIXME: should be date
    pub title: Option<String>,
    pub muxing_app: String,
    pub writing_app: String,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.8
pub fn info(input: &[u8]) -> EbmlResult<SegmentElement> {
    matroska_permutation((
        uuid(0x73A4),                // SegmentUID
        str(0x7384),                 // SegmentFIlename FIXME SHOULD BE UTF-8 not str
        uuid(0x3CB923),              // PrevUID
        str(0x3C83AB),               // PrevFilename FIXME SHOULD BE UTF-8 not str
        uuid(0x3EB923),              // NextUID
        str(0x3E83BB),               // NextFilename FIXME SHOULD BE UTF-8 not str
        uuid(0x4444),                // SegmentFamily
        complete(chapter_translate), //
        uint(0x2AD7B1),              // TimecodeScale
        float(0x4489),               // Duration: FIXME should be float
        binary(0x4461),              // DateUTC FIXME: should be date
        str(0x7BA9),                 // Title FIXME SHOULD BE UTF-8 not str
        str(0x4D80),                 // MuxingApp FIXME SHOULD BE UTF-8 not str
        str(0x5741),                 // WritingApp FIXME SHOULD BE UTF-8 not str
    ))(input)
    .and_then(|(i, t)| {
        Ok((
            i,
            SegmentElement::Info(Info {
                segment_uid: t.0,
                segment_filename: t.1,
                prev_uid: t.2,
                prev_filename: t.3,
                next_uid: t.4,
                next_filename: t.5,
                segment_family: t.6,
                chapter_translate: t.7,
                timecode_scale: value_error(0x2AD7B1, t.8)?,
                duration: t.9,
                date_utc: t.10,
                title: t.11,
                muxing_app: value_error(0x4D80, t.12)?,
                writing_app: value_error(0x5741, t.13)?,
            }),
        ))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterTranslate {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
pub fn chapter_translate(input: &[u8]) -> EbmlResult<ChapterTranslate> {
    master(0x6924, |i| Ok((i, ChapterTranslate {})))(input)
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.26
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cluster<'a> {
    pub timecode: u64,
    pub silent_tracks: Option<SilentTracks>,
    pub position: Option<u64>,
    pub prev_size: Option<u64>,
    pub simple_block: Vec<&'a [u8]>,
    pub block_group: Vec<BlockGroup<'a>>,
    pub encrypted_block: Option<&'a [u8]>,
}

pub fn cluster(input: &[u8]) -> EbmlResult<SegmentElement> {
    matroska_permutation((
        uint(0xE7),
        complete(silent_tracks),
        uint(0xA7),
        uint(0xAB),
        many0(binary_ref(0xA3)),
        many0(complete(block_group)),
        binary_ref(0xAF),
    ))(input)
    .and_then(|(i, t)| {
        Ok((
            i,
            SegmentElement::Cluster(Cluster {
                timecode: value_error(0xE7, t.0)?,
                silent_tracks: t.1,
                position: t.2,
                prev_size: t.3,
                simple_block: value_error(0xA3, t.4)?,
                block_group: value_error(0xA0, t.5)?,
                encrypted_block: t.6,
            }),
        ))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SilentTracks {
    pub numbers: Vec<u64>,
}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
pub fn silent_tracks(input: &[u8]) -> EbmlResult<SilentTracks> {
    master(0x5854, |i| {
        map(many0(uint(0x58D7)), |v| SilentTracks { numbers: v })(i)
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
pub fn block_group(input: &[u8]) -> EbmlResult<BlockGroup> {
    master(0xA0, |inp| {
        matroska_permutation((
            binary_ref(0xA1),
            binary(0xA2),
            complete(block_additions),
            uint(0x9B),
            uint(0xFA),
            uint(0xFB),
            int(0xFD),
            binary(0xA4),
            int(0x75A2),
            complete(slices),
            complete(reference_frame),
        ))(inp)
        .and_then(|(i, t)| {
            Ok((
                i,
                BlockGroup {
                    block: value_error(0xA1, t.0)?,
                    block_virtual: t.1,
                    block_additions: t.2,
                    block_duration: t.3,
                    reference_priority: value_error(0xFA, t.4).unwrap_or(0),
                    reference_block: t.5,
                    reference_virtual: t.6,
                    codec_state: t.7,
                    discard_padding: t.8,
                    slices: t.9,
                    reference_frame: t.10,
                },
            ))
        })
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockAdditions {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.16
pub fn block_additions(input: &[u8]) -> EbmlResult<BlockAdditions> {
    master(0x75A1, |i| Ok((i, BlockAdditions {})))(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slices {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.46
pub fn slices(input: &[u8]) -> EbmlResult<Slices> {
    master(0x8E, |i| Ok((i, Slices {})))(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceFrame {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.53
pub fn reference_frame(input: &[u8]) -> EbmlResult<ReferenceFrame> {
    master(0xC8, |i| Ok((i, ReferenceFrame {})))(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub track_number: u64,
    pub timecode: i16,
    pub invisible: bool,
    pub lacing: Lacing,
}

pub fn block(input: &[u8]) -> EbmlResult<Block> {
    map(
        tuple((vint, be_i16, map_opt(be_u8, block_flags))),
        |(track_number, timecode, flags)| Block {
            track_number,
            timecode,
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
        lacing,
        discardable: (data & (1 << 7)) != 0,
    })
}

pub fn simple_block(input: &[u8]) -> EbmlResult<SimpleBlock> {
    map(
        tuple((vint, be_i16, map_opt(be_u8, block_flags))),
        |(track_number, timecode, flags)| SimpleBlock {
            track_number,
            timecode,
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
pub fn tracks(input: &[u8]) -> EbmlResult<SegmentElement> {
    map(many1(complete(skip_void(track_entry))), |v| {
        SegmentElement::Tracks(Tracks { tracks: v })
    })(input)
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
    pub trick_track_segment_uid: Option<Uuid>,
    pub trick_track_flag: Option<u64>,
    pub trick_master_track_uid: Option<u64>,
    pub trick_master_track_segment_uid: Option<Uuid>,
    pub video: Option<Video>,
    pub audio: Option<Audio>,
    pub track_translate: Vec<TrackTranslate>,
    pub track_operation: Option<TrackOperation>,
    pub content_encodings: Option<ContentEncodings>,
    /// The demuxer Stream index matching the Track
    pub stream_index: usize,
}

pub fn track_entry(input: &[u8]) -> EbmlResult<TrackEntry> {
    master(0xAE, |inp| {
        matroska_permutation((
            uint(0xD7),
            uint(0x73C5),
            uint(0x83),
            uint(0xB9),
            uint(0x88),
            uint(0x55AA),
            uint(0x9C),
            uint(0x6DE7),
            uint(0x6DF8),
            uint(0x23E383),
            uint(0x234E7A),
            float(0x23314F),
            int(0x537F),
            uint(0x55EE),
            str(0x536E),
            str(0x22B59C),
            str(0x22B59D),
            str(0x86),
            binary(0x63A2),
            str(0x258688),
            uint(0x7446),
            str(0x3A9697),
            str(0x3B4040),
            str(0x26B240),
            uint(0xAA),
            uint(0x6FAB),
            uint(0x56AA),
            uint(0x56BB),
            many0(complete(track_translate)),
            complete(video),
            complete(audio),
            complete(track_operation),
            uint(0xC0),
            uuid(0xC1),
            uint(0xC6),
            uint(0xC7),
            uuid(0xC4),
            complete(content_encodings),
        ))(inp)
        .and_then(|(i, t)| {
            Ok((
                i,
                TrackEntry {
                    track_number: value_error(0xD7, t.0)?,
                    track_uid: value_error(0x73C5, t.1)?,
                    track_type: value_error(0x83, t.2)?,
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
                    codec_id: value_error(0x86, t.17)?,
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
                    track_translate: value_error(0x6624, t.28)?,
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
                },
            ))
        })
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackTranslate {
    pub edition_uid: Vec<u64>,
    pub codec: u64,
    pub track_id: u64,
}

pub fn track_translate(input: &[u8]) -> EbmlResult<TrackTranslate> {
    master(0x6624, |inp| {
        matroska_permutation((many1(uint(0x66FC)), uint(0x66BF), uint(0x66A5)))(inp).and_then(
            |(i, t)| {
                Ok((
                    i,
                    TrackTranslate {
                        edition_uid: value_error(0x66FC, t.0)?,
                        codec: value_error(0x66BF, t.1)?,
                        track_id: value_error(0x66A5, t.2)?,
                    },
                ))
            },
        )
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackOperation {
    pub combine_planes: Option<TrackCombinePlanes>,
    pub join_blocks: Option<TrackJoinBlocks>,
}

pub fn track_operation(input: &[u8]) -> EbmlResult<TrackOperation> {
    master(0xE2, |i| {
        map(
            matroska_permutation((complete(track_combine_planes), complete(track_join_blocks))),
            |t| TrackOperation {
                combine_planes: t.0,
                join_blocks: t.1,
            },
        )(i)
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackCombinePlanes {
    pub track_planes: Vec<TrackPlane>,
}

pub fn track_combine_planes(input: &[u8]) -> EbmlResult<TrackCombinePlanes> {
    master(0xE3, |i| {
        map(many1(complete(track_plane)), |v| TrackCombinePlanes {
            track_planes: v,
        })(i)
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackPlane {
    pub uid: u64,
    pub plane_type: u64,
}

pub fn track_plane(input: &[u8]) -> EbmlResult<TrackPlane> {
    master(0xE4, |inp| {
        matroska_permutation((uint(0xE5), uint(0xE6)))(inp).and_then(|(i, t)| {
            Ok((
                i,
                TrackPlane {
                    uid: value_error(0xE5, t.0)?,
                    plane_type: value_error(0xE6, t.1)?,
                },
            ))
        })
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackJoinBlocks {
    pub uid: Vec<u64>,
}

pub fn track_join_blocks(input: &[u8]) -> EbmlResult<TrackJoinBlocks> {
    master(0xE9, |i| {
        map(many1(uint(0xED)), |v| TrackJoinBlocks { uid: v })(i)
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentEncodings {
    pub content_encoding: Vec<ContentEncoding>,
}

pub fn content_encodings(input: &[u8]) -> EbmlResult<ContentEncodings> {
    master(0x6D80, |i| {
        map(many1(complete(content_encoding)), |v| ContentEncodings {
            content_encoding: v,
        })(i)
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentEncoding {
    order: u64,
    scope: u64,
    encoding_type: u64,
    compression: Option<ContentCompression>,
    encryption: Option<ContentEncryption>,
}

pub fn content_encoding(input: &[u8]) -> EbmlResult<ContentEncoding> {
    master(0x6240, |inp| {
        matroska_permutation((
            uint(0x5031),
            uint(0x5032),
            uint(0x5033),
            complete(content_compression),
            complete(content_encryption),
        ))(inp)
        .and_then(|(i, t)| {
            Ok((
                i,
                ContentEncoding {
                    order: value_error(0x5031, t.0)?,
                    scope: value_error(0x5032, t.1)?,
                    encoding_type: value_error(0x5033, t.2)?,
                    compression: t.3,
                    encryption: t.4,
                },
            ))
        })
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentCompression {
    algo: u64,
    settings: Option<u64>,
}

pub fn content_compression(input: &[u8]) -> EbmlResult<ContentCompression> {
    master(0x5034, |inp| {
        matroska_permutation((uint(0x4254), uint(0x4255)))(inp).and_then(|(i, t)| {
            Ok((
                i,
                ContentCompression {
                    algo: value_error(0x4254, t.0)?,
                    settings: t.1,
                },
            ))
        })
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentEncryption {
    enc_algo: Option<u64>,
    enc_key_id: Option<Vec<u8>>,
    signature: Option<Vec<u8>>,
    sig_key_id: Option<Vec<u8>>,
    sig_algo: Option<u64>,
    sig_hash_algo: Option<u64>,
}

pub fn content_encryption(input: &[u8]) -> EbmlResult<ContentEncryption> {
    master(0x5035, |i| {
        map(
            matroska_permutation((
                uint(0x47E1),
                binary(0x47E2),
                binary(0x47E3),
                binary(0x47E4),
                uint(0x47E5),
                uint(0x47E6),
            )),
            |t| ContentEncryption {
                enc_algo: t.0,
                enc_key_id: t.1,
                signature: t.2,
                sig_key_id: t.3,
                sig_algo: t.4,
                sig_hash_algo: t.5,
            },
        )(i)
    })(input)
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Audio {
    pub sampling_frequency: f64,
    pub output_sampling_frequency: Option<f64>,
    pub channels: u64,
    pub channel_positions: Option<Vec<u8>>,
    pub bit_depth: Option<u64>,
}

pub fn audio(input: &[u8]) -> EbmlResult<Audio> {
    master(0xE1, |inp| {
        matroska_permutation((
            float(0xB5),
            float(0x78B5),
            uint(0x9F),
            binary(0x7D7B),
            uint(0x6264),
        ))(inp)
        .and_then(|(i, t)| {
            Ok((
                i,
                Audio {
                    sampling_frequency: value_error(0xB5, t.0)?,
                    output_sampling_frequency: t.1,
                    channels: value_error(0x9F, t.2)?,
                    channel_positions: t.3,
                    bit_depth: t.4,
                },
            ))
        })
    })(input)
}

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

pub fn video(input: &[u8]) -> EbmlResult<Video> {
    master(0xE0, |inp| {
        matroska_permutation((
            uint(0x9A),
            uint(0x9D),
            uint(0x53B8),
            uint(0x53C0),
            uint(0x53B9),
            uint(0xB0),
            uint(0xBA),
            uint(0x54AA),
            uint(0x54BB),
            uint(0x54CC),
            uint(0x54DD),
            uint(0x54B0),
            uint(0x54BA),
            uint(0x54B2),
            uint(0x54B3),
            binary(0x2EB524),
            float(0x2FB523),
            float(0x2383E3),
            complete(colour),
            complete(projection),
        ))(inp)
        .and_then(|(i, t)| {
            Ok((
                i,
                Video {
                    flag_interlaced: t.0,
                    field_order: t.1,
                    stereo_mode: t.2,
                    alpha_mode: t.3,
                    old_stereo_mode: t.4,
                    pixel_width: value_error(0xB0, t.5)?,
                    pixel_height: value_error(0xBA, t.6)?,
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
                },
            ))
        })
    })(input)
}

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

pub fn colour(input: &[u8]) -> EbmlResult<Colour> {
    master(0x55B0, |i| {
        map(
            matroska_permutation((
                uint(0x55B1),
                uint(0x55B2),
                uint(0x55B3),
                uint(0x55B4),
                uint(0x55B5),
                uint(0x55B6),
                uint(0x55B7),
                uint(0x55B8),
                uint(0x55B9),
                uint(0x55BA),
                uint(0x55BB),
                uint(0x55BC),
                uint(0x55BD),
                complete(mastering_metadata),
            )),
            |t| Colour {
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
            },
        )(i)
    })(input)
}

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

pub fn mastering_metadata(input: &[u8]) -> EbmlResult<MasteringMetadata> {
    master(0x55D0, |i| {
        map(
            matroska_permutation((
                float(0x55D1),
                float(0x55D2),
                float(0x55D3),
                float(0x55D4),
                float(0x55D5),
                float(0x55D6),
                float(0x55D7),
                float(0x55D8),
                float(0x55D9),
                float(0x55DA),
            )),
            |t| MasteringMetadata {
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
            },
        )(i)
    })(input)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    pub projection_type: u64,
    pub projection_private: Option<Vec<u8>>,
    pub projection_pose_yaw: f64,
    pub projection_pose_pitch: f64,
    pub projection_pose_roll: f64,
}

pub fn projection(input: &[u8]) -> EbmlResult<Projection> {
    master(0x7670, |inp| {
        matroska_permutation((
            uint(0x7671),
            binary(0x7672),
            float(0x7673),
            float(0x7674),
            float(0x7675),
        ))(inp)
        .and_then(|(i, t)| {
            Ok((
                i,
                Projection {
                    projection_type: value_error(0x7671, t.0)?,
                    projection_private: t.1,
                    projection_pose_yaw: value_error(0x7673, t.2)?,
                    projection_pose_pitch: value_error(0x7674, t.3)?,
                    projection_pose_roll: value_error(0x7675, t.4)?,
                },
            ))
        })
    })(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapters {}

//https://datatracker.ietf.org/doc/html/draft-lhomme-cellar-matroska-03#section-7.3.199
pub fn chapters(input: &[u8]) -> EbmlResult<SegmentElement> {
    master(0x45B9, |i| Ok((i, SegmentElement::Chapters(Chapters {}))))(input)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cues {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachments {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tags {}

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use std::cmp::min;

    use log::debug;
    use nom::{HexDisplay, Offset};

    use super::*;

    const mkv: &[u8] = include_bytes!("../assets/single_stream.mkv");
    const webm: &[u8] = include_bytes!("../assets/big-buck-bunny_trailer.webm");

    #[test]
    fn mkv_segment_root() {
        let res = segment(&mkv[47..100]);
        debug!("{:?}", res);

        if let Ok((i, _)) = res {
            debug!("consumed {} bytes after header", (mkv[47..]).offset(i));
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

                    index = new_index;
                }
                e => {
                    let max_index = min(mkv.len(), index + 200);
                    debug!(
                        "[{}] {:#?}:\n{}",
                        index,
                        e,
                        (mkv[index..max_index]).to_hex(16)
                    );
                    break;
                }
            }
        }
    }

    #[test]
    fn webm_segment_root() {
        let res = segment(&webm[40..100]);
        debug!("{:?}", res);

        if let Ok((i, _)) = res {
            debug!("consumed {} bytes after header", (webm[40..]).offset(i));
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

                    index = new_index;
                }
                e => {
                    let max_index = min(webm.len(), index + 200);
                    debug!(
                        "[{}] {:#?}:\n{}",
                        index,
                        e,
                        (webm[index..max_index]).to_hex(16)
                    );
                    break;
                }
            }
        }
    }
}
