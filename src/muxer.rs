use std::io::Write;
use std::sync::Arc;

use cookie_factory::GenError;
use log::error;

use av_data::{packet::Packet, params::MediaKind, value::Value};
use av_format::{common::GlobalInfo, error::*, muxer::*, stream::Stream};

use crate::{
    ebml::EBMLHeader,
    elements::{
        Audio, Cluster, BlockGroup, Colour, Info, Lacing, Seek, SeekHead, SimpleBlock, TrackEntry, TrackType,
        Tracks, Video,
    },
    serializer::{
        cookie_utils::tuple,
        ebml::{gen_ebml_header, EbmlSize},
        elements::{
            gen_cluster, gen_info, gen_seek_head, gen_segment_header_unknown_size,
            gen_simple_block_header, gen_tracks,
        },
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct MkvMuxer {
    header: EBMLHeader,
    seek_head: SeekHead,
    info: Option<Info>,
    tracks: Option<Tracks>,
    blocks: Vec<Vec<u8>>,
    blocks_len: usize,
    timecode: Option<u64>,
}

impl MkvMuxer {
    pub fn matroska() -> MkvMuxer {
        MkvMuxer {
            header: EBMLHeader {
                version: 1,
                read_version: 1,
                max_id_length: 4,
                max_size_length: 8,
                doc_type: String::from("matroska"),
                doc_type_version: 4,
                doc_type_read_version: 2,
            },
            seek_head: SeekHead {
                positions: Vec::new(),
            },
            info: None,
            tracks: None,
            blocks: Vec::new(),
            blocks_len: 0,
            timecode: None,
        }
    }

    pub fn webm() -> MkvMuxer {
        MkvMuxer {
            header: EBMLHeader {
                version: 1,
                read_version: 1,
                max_id_length: 4,
                max_size_length: 8,
                doc_type: String::from("webm"),
                doc_type_version: 1,
                doc_type_read_version: 1,
            },
            seek_head: SeekHead {
                positions: Vec::new(),
            },
            info: None,
            tracks: None,
            blocks: Vec::new(),
            blocks_len: 0,
            timecode: None,
        }
    }

    pub fn write_ebml_header(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        let mut origin = (buf).as_ptr() as usize;

        let mut needed = 0usize;
        let offset;
        loop {
            if needed > 0 {
                let len = needed + buf.len();
                buf.resize(len, 0);
                origin = (buf).as_ptr() as usize;
            }

            match gen_ebml_header(&self.header)((buf, 0)) {
                Err(GenError::BufferTooSmall(sz)) => {
                    needed = sz;
                }
                Err(e) => {
                    error!("{:?}", e);
                    return Err(Error::InvalidData);
                }
                Ok((sl, sz)) => {
                    offset = sl.as_ptr() as usize + sz - origin;
                    break;
                }
            };
        }
        buf.truncate(offset);

        Ok(())
    }

    pub fn write_segment_header(&mut self, buf: &mut Vec<u8>, _size: usize) -> Result<()> {
        let mut origin = (buf).as_ptr() as usize;

        let mut needed = 0usize;
        let offset;
        loop {
            if needed > 0 {
                let len = needed + buf.len();
                buf.resize(len, 0);
                origin = (buf).as_ptr() as usize;
            }

            match gen_segment_header_unknown_size()((buf, 0)) {
                Err(GenError::BufferTooSmall(sz)) => {
                    needed = sz;
                }
                Err(e) => {
                    error!("{:?}", e);
                    return Err(Error::InvalidData);
                }
                Ok((sl, sz)) => {
                    offset = sl.as_ptr() as usize + sz - origin;
                    break;
                }
            };
        }
        buf.truncate(offset);

        Ok(())
    }

    pub fn write_seek_head(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        let mut origin = (buf).as_ptr() as usize;

        let mut needed = 0usize;
        let offset;
        loop {
            if needed > 0 {
                let len = needed + buf.len();
                buf.resize(len, 0);
                origin = (buf).as_ptr() as usize;
            }

            match gen_seek_head(&self.seek_head)((buf, 0)) {
                Err(GenError::BufferTooSmall(sz)) => {
                    needed = sz;
                }
                Err(e) => {
                    error!("{:?}", e);
                    return Err(Error::InvalidData);
                }
                Ok((sl, sz)) => {
                    offset = sl.as_ptr() as usize + sz - origin;
                    break;
                }
            };
        }
        buf.truncate(offset);

        Ok(())
    }

    pub fn write_info(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        if let Some(info) = self.info.as_ref() {
            let mut origin = (buf).as_ptr() as usize;
            let mut needed = 0usize;
            let offset;
            loop {
                if needed > 0 {
                    let len = needed + buf.len();
                    buf.resize(len, 0);
                    origin = (buf).as_ptr() as usize;
                }

                match gen_info(info)((buf, 0)) {
                    Err(GenError::BufferTooSmall(sz)) => {
                        needed = sz;
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        return Err(Error::InvalidData);
                    }
                    Ok((sl, sz)) => {
                        offset = sl.as_ptr() as usize + sz - origin;
                        break;
                    }
                };
            }
            buf.truncate(offset);
        }
        Ok(())
    }

    pub fn write_tracks(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        if let Some(tracks) = self.tracks.as_ref() {
            let mut origin = (buf).as_ptr() as usize;
            let mut needed = 0usize;
            let offset;
            loop {
                if needed > 0 {
                    let len = needed + buf.len();
                    buf.resize(len, 0);
                    origin = (buf).as_ptr() as usize;
                }

                match gen_tracks(tracks)((buf, 0)) {
                    Err(GenError::BufferTooSmall(sz)) => {
                        needed = sz;
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        return Err(Error::InvalidData);
                    }
                    Ok((sl, sz)) => {
                        offset = sl.as_ptr() as usize + sz - origin;
                        break;
                    }
                };
            }
            buf.truncate(offset);
        }
        Ok(())
    }
}

impl Muxer for MkvMuxer {
    fn configure(&mut self) -> Result<()> {
        Ok(())
    }

    fn write_header<W: Write>(&mut self, out: &mut Writer<W>) -> Result<()> {
        let mut buf = Vec::new();
        let mut ebml_header = Vec::new();
        self.write_ebml_header(&mut ebml_header)?;
        let mut segment_header = Vec::new();
        //127 corresponds to unknown size
        self.write_segment_header(&mut segment_header, 127)?;

        buf.extend_from_slice(&ebml_header);
        buf.extend_from_slice(&segment_header);

        let mut info = Vec::new();
        self.write_info(&mut info)?;
        let mut tracks = Vec::new();
        self.write_tracks(&mut tracks)?;

        let info_seek = Seek {
            id: vec![0x15, 0x49, 0xA9, 0x66],
            position: 0,
        };
        let tracks_seek = Seek {
            id: vec![0x16, 0x54, 0xAE, 0x6B],
            position: 0,
        };
        let cluster_seek = Seek {
            id: vec![0x1F, 0x43, 0xB6, 0x75],
            position: 0,
        };
        self.seek_head.positions.push(info_seek);
        self.seek_head.positions.push(tracks_seek);
        self.seek_head.positions.push(cluster_seek);

        self.seek_head.positions[0].position = self.seek_head.size(0x114D9B74) as u64;
        self.seek_head.positions[1].position =
            (self.seek_head.size(0x114D9B74) + info.size(0x1549A966)) as u64;
        self.seek_head.positions[2].position = (self.seek_head.size(0x114D9B74)
            + info.size(0x1549A966)
            + tracks.size(0x1654AE6B)) as u64;

        let mut seek_head = Vec::new();
        self.write_seek_head(&mut seek_head)?;

        buf.extend_from_slice(&seek_head);
        buf.extend_from_slice(&info);
        buf.extend_from_slice(&tracks);

        out.write_all(&buf).unwrap();

        Ok(())
    }

    fn write_packet<W: Write>(&mut self, out: &mut Writer<W>, pkt: Arc<Packet>) -> Result<()> {
        let mut v = Vec::with_capacity(16);

        let s = SimpleBlock {
            track_number: pkt.stream_index as u64 + 1,
            timecode: pkt.t.pts.or(pkt.t.dts).unwrap_or(0) as i16,
            keyframe: pkt.is_key,
            invisible: false,
            lacing: Lacing::None,
            discardable: false,
        };

        let mut origin = (&v).as_ptr() as usize;
        let mut needed = 0usize;
        let offset;
        loop {
            if needed > 0 {
                let len = needed + v.len();
                v.resize(len, 0);
                origin = (&v).as_ptr() as usize;
            }

            match gen_simple_block_header(&s)((&mut v, 0)) {
                Err(GenError::BufferTooSmall(sz)) => {
                    needed = sz;
                }
                Err(e) => {
                    error!("{:?}", e);
                    return Err(Error::InvalidData);
                }
                Ok((sl, sz)) => {
                    offset = sl.as_ptr() as usize + sz - origin;
                    break;
                }
            };
        }
        v.truncate(offset);

        v.extend(pkt.data.iter());
        let len = v.len();
        self.blocks.push(v);
        self.blocks_len += len;

        self.timecode = if self.timecode.is_none() {
            Some(pkt.t.pts.or(pkt.t.dts).unwrap_or(0) as u64)
        } else {
            return Err(Error::InvalidData);
        };

        if pkt.is_key || self.blocks_len >= 5242880 {
            {
                let simple_blocks: Vec<&[u8]> = self.blocks.iter().map(|v| &v[..]).collect();

                let cluster = Cluster {
                    timecode: self.timecode.take().unwrap(),
                    silent_tracks: None,
                    position: None,
                    prev_size: None,
                    block: simple_blocks.iter().map(|block| BlockGroup{
                        block,
                        block_virtual: None,
                        block_additions: None,
                        block_duration: None,
                        reference_priority: 0,
                        reference_block: None,
                        reference_virtual: None,
                        codec_state: None,
                        discard_padding: None,
                        slices: None,
                        reference_frame: None
                    }).collect(),
                    encrypted_block: None,
                };

                let mut buf: Vec<u8> = vec![0; cluster.size(0x1F43B675)];
                let mut origin = (&buf).as_ptr() as usize;
                let mut needed = 0usize;
                let offset;
                loop {
                    if needed > 0 {
                        let len = needed + buf.len();
                        buf.resize(len, 0);
                        origin = (&buf).as_ptr() as usize;
                    }

                    match gen_cluster(&cluster)((&mut buf, 0)) {
                        Err(GenError::BufferTooSmall(sz)) => {
                            needed = sz;
                        }
                        Err(e) => {
                            error!("{:?}", e);
                            return Err(Error::InvalidData);
                        }
                        Ok((sl, sz)) => {
                            offset = sl.as_ptr() as usize + sz - origin;
                            break;
                        }
                    };
                }
                buf.truncate(offset);
                out.write_all(&buf).unwrap();
            }

            self.blocks.truncate(0);
            self.blocks_len = 0;
        }

        Ok(())
    }

    fn write_trailer<W: Write>(&mut self, out: &mut Writer<W>) -> Result<()> {
        let nb = self.blocks.len();

        if nb > 0 {
            let simple_blocks: Vec<&[u8]> = self.blocks.iter().map(|v| &v[..]).collect();

            let cluster = Cluster {
                timecode: self.timecode.take().unwrap(),
                silent_tracks: None,
                position: None,
                prev_size: None,
                block: simple_blocks.iter().map(|block| BlockGroup{
                    block,
                    block_virtual: None,
                    block_additions: None,
                    block_duration: None,
                    reference_priority: 0,
                    reference_block: None,
                    reference_virtual: None,
                    codec_state: None,
                    discard_padding: None,
                    slices: None,
                    reference_frame: None
                }).collect(),
                encrypted_block: None,
            };

            let mut buf: Vec<u8> = vec![0; cluster.size(0x1F43B675)];
            let mut origin = (&buf).as_ptr() as usize;
            let mut needed = 0usize;
            let offset;
            loop {
                if needed > 0 {
                    let len = needed + buf.len();
                    buf.resize(len, 0);
                    origin = (&buf).as_ptr() as usize;
                }

                match gen_cluster(&cluster)((&mut buf, 0)) {
                    Err(GenError::BufferTooSmall(sz)) => {
                        needed = sz;
                    }
                    Err(e) => {
                        error!("{:?}", e);
                        return Err(Error::InvalidData);
                    }
                    Ok((sl, sz)) => {
                        offset = sl.as_ptr() as usize + sz - origin;
                        break;
                    }
                };
            }
            buf.truncate(offset);
            out.write_all(&buf).unwrap();
        }

        Ok(())
    }

    fn set_global_info(&mut self, info: GlobalInfo) -> Result<()> {
        self.tracks = Some(Tracks {
            tracks: info.streams.iter().map(stream_to_track).collect(),
        });

        self.info = Some(Info {
            muxing_app: String::from("rust-av"),
            writing_app: String::from("rust-av"),
            duration: info.duration.map(|d| d as f64),
            timecode_scale: 1000000,
            ..Default::default()
        });

        Ok(())
    }

    fn set_option<'a>(&mut self, _key: &str, _val: Value<'a>) -> Result<()> {
        Ok(())
    }
}

#[allow(dead_code)]
fn offset<'a>(original: &(&'a [u8], usize), subslice: &(&'a [u8], usize)) -> usize {
    let first = original.0.as_ptr() as usize;
    let second = subslice.0.as_ptr() as usize;

    second + subslice.1 - first - original.1
}

#[allow(dead_code)]
fn gen_mkv_prefix<'b>(
    input: (&'b mut [u8], usize),
    header: &EBMLHeader,
    seek_head: &SeekHead,
    info: &Info,
    tracks: &Tracks,
) -> std::result::Result<(&'b mut [u8], usize), GenError> {
    tuple((
        gen_ebml_header(header),
        gen_seek_head(seek_head),
        gen_info(info),
        gen_tracks(tracks),
    ))(input)
}

pub fn stream_to_track(s: &Stream) -> TrackEntry {
    let codec_id = match s.params.codec_id.as_deref() {
        Some("opus") => String::from("A_OPUS"),
        Some("vorbis") => String::from("A_VORBIS"),
        Some("av1") => String::from("V_AV1"),
        Some("vp8") => String::from("V_VP8"),
        Some("vp9") => String::from("V_VP9"),
        _ => String::from("INVALID_CODEC"),
    };

    let mut t = TrackEntry {
        track_uid: s.id as u64,
        track_number: s.index as u64 + 1,
        track_type: 0,
        codec_id,
        default_duration: s.duration,
        codec_delay: Some(s.params.delay as u64),
        codec_private: s.params.extradata.clone(),
        seek_pre_roll: Some(s.params.convergence_window as u64),
        ..Default::default()
    };

    match s.params.kind {
        Some(MediaKind::Video(ref v)) => {
            t.track_type = TrackType::Video.into();
            t.video = Some(Video {
                pixel_width: v.width as u64,
                pixel_height: v.height as u64,
                colour: Some(Colour {
                    matrix_coefficients: Some(match v.format.as_ref() {
                        Some(fmt) => fmt.get_matrix() as u64,
                        None => 0u64,
                    }),
                    transfer_characteristics: Some(match v.format.as_ref() {
                        Some(fmt) => fmt.get_xfer() as u64,
                        None => 0u64,
                    }),
                    primaries: Some(match v.format.as_ref() {
                        Some(fmt) => fmt.get_primaries() as u64,
                        None => 0u64,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        Some(MediaKind::Audio(ref a)) => {
            t.track_type = TrackType::Audio.into();
            t.audio = Some(Audio {
                sampling_frequency: a.rate as f64,
                channels: 1,
                ..Default::default()
            });
        }
        _ => {}
    }

    t
}
