mod error;
pub(crate) mod macros;
mod parse;

#[cfg(test)]
mod tests;

pub use self::error::{ebml_err, Error, ErrorKind};
pub use self::parse::*;

self::macros::impl_ebml_master! {
    // Element ID 0x1A45DFA3
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct EbmlHeader {
        [0x4286] version: (u32) = 1,
        [0x42F7] read_version: (u32) = 1,
        [0x42F2] max_id_length: (u32) = 4,
        [0x42F3] max_size_length: (u32) = 8,
        [0x4282] doc_type: (String),
        [0x4287] doc_type_version: (u32) = 1,
        [0x4285] doc_type_read_version: (u32) = 1,
    }
}

pub fn ebml_header(input: &[u8]) -> EbmlResult<EbmlHeader> {
    ebml_element(0x1A45DFA3)(input)
}

/// Map of known deprecated Element IDs and their EBML Paths
pub static DEPRECATED: phf::Map<u32, &'static str> = phf::phf_map! {
    0x8E_u32 => r"\Segment\Cluster\BlockGroup\Slices",
    0x97_u32 => r"\Segment\Cues\CuePoint\CueTrackPositions\CueReference\CueRefCluster",
    0xA2_u32 => r"\Segment\Cluster\BlockGroup\BlockVirtual",
    0xAA_u32 => r"\Segment\Tracks\TrackEntry\CodecDecodeAll",
    0xAF_u32 => r"\Segment\Cluster\EncryptedBlock",
    0xC0_u32 => r"\Segment\Tracks\TrackEntry\TrickTrackUID",
    0xC1_u32 => r"\Segment\Tracks\TrackEntry\TrickTrackSegmentUID",
    0xC4_u32 => r"\Segment\Tracks\TrackEntry\TrickMasterTrackSegmentUID",
    0xC6_u32 => r"\Segment\Tracks\TrackEntry\TrickTrackFlag",
    0xC7_u32 => r"\Segment\Tracks\TrackEntry\TrickMasterTrackUID",
    0xC8_u32 => r"\Segment\Cluster\BlockGroup\ReferenceFrame",
    0xC9_u32 => r"\Segment\Cluster\BlockGroup\ReferenceFrame\ReferenceOffset",
    0xCA_u32 => r"\Segment\Cluster\BlockGroup\ReferenceFrame\ReferenceTimestamp",
    0xCB_u32 => r"\Segment\Cluster\BlockGroup\Slices\TimeSlice\BlockAdditionID",
    0xCC_u32 => r"\Segment\Cluster\BlockGroup\Slices\TimeSlice\LaceNumber",
    0xCD_u32 => r"\Segment\Cluster\BlockGroup\Slices\TimeSlice\FrameNumber",
    0xCE_u32 => r"\Segment\Cluster\BlockGroup\Slices\TimeSlice\Delay",
    0xCF_u32 => r"\Segment\Cluster\BlockGroup\Slices\TimeSlice\SliceDuration",
    0xE8_u32 => r"\Segment\Cluster\BlockGroup\Slices\TimeSlice",
    0xEB_u32 => r"\Segment\Cues\CuePoint\CueTrackPositions\CueReference\CueRefCodecState",
    0xFD_u32 => r"\Segment\Cluster\BlockGroup\ReferenceVirtual",
    0x44B4_u32 => r"\Segment\Tags\Tag\+SimpleTag\TagDefaultBogus",
    0x4661_u32 => r"\Segment\Attachments\AttachedFile\FileUsedStartTime",
    0x4662_u32 => r"\Segment\Attachments\AttachedFile\FileUsedEndTime",
    0x4675_u32 => r"\Segment\Attachments\AttachedFile\FileReferral",
    0x47E3_u32 => r"\Segment\Tracks\TrackEntry\ContentEncodings\ContentEncoding\ContentEncryption\ContentSignature",
    0x47E4_u32 => r"\Segment\Tracks\TrackEntry\ContentEncodings\ContentEncoding\ContentEncryption\ContentSigKeyID",
    0x47E5_u32 => r"\Segment\Tracks\TrackEntry\ContentEncodings\ContentEncoding\ContentEncryption\ContentSigAlgo",
    0x47E6_u32 => r"\Segment\Tracks\TrackEntry\ContentEncodings\ContentEncoding\ContentEncryption\ContentSigHashAlgo",
    0x535F_u32 => r"\Segment\Cues\CuePoint\CueTrackPositions\CueReference\CueRefNumber",
    0x537F_u32 => r"\Segment\Tracks\TrackEntry\TrackOffset",
    0x54B3_u32 => r"\Segment\Tracks\TrackEntry\Video\AspectRatioType",
    0x5854_u32 => r"\Segment\Cluster\SilentTracks",
    0x58D7_u32 => r"\Segment\Cluster\SilentTracks\SilentTrackNumber",
    0x6DE7_u32 => r"\Segment\Tracks\TrackEntry\MinCache",
    0x6DF8_u32 => r"\Segment\Tracks\TrackEntry\MaxCache",
    0x6FAB_u32 => r"\Segment\Tracks\TrackEntry\TrackOverlay",
    0x7D7B_u32 => r"\Segment\Tracks\TrackEntry\Audio\ChannelPositions",
    0x2383E3_u32 => r"\Segment\Tracks\TrackEntry\Video\FrameRate",
    0x26B240_u32 => r"\Segment\Tracks\TrackEntry\CodecDownloadURL",
    0x2FB523_u32 => r"\Segment\Tracks\TrackEntry\Video\GammaValue",
    0x3A9697_u32 => r"\Segment\Tracks\TrackEntry\CodecSettings",
    0x3B4040_u32 => r"\Segment\Tracks\TrackEntry\CodecInfoURL",
};
