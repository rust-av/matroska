use std::{collections::VecDeque, io::SeekFrom};

use log::{debug, error, trace};
use nom::{self, Err, IResult, Needed, Offset};

use av_data::{packet::Packet, params::*, rational::Rational64, timeinfo::TimeInfo};
use av_format::{
    buffer::Buffered,
    common::GlobalInfo,
    demuxer::{Demuxer, Descr, Descriptor, Event},
    error::*,
    stream::Stream,
};

use crate::{
    ebml::{self, custom_error, ebml_header, EBMLHeader},
    elements::{
        segment, segment_element, simple_block, Cluster, Info, SeekHead, SegmentElement,
        TrackEntry, Tracks,
    },
};

#[derive(Debug, Clone, Default)]
pub struct MkvDemuxer {
    pub header: Option<EBMLHeader>,
    pub seek_head: Option<SeekHead>,
    pub info: Option<Info>,
    pub tracks: Option<Tracks>,
    pub queue: VecDeque<Event>,
    pub blockstream: Vec<u8>,
    pub params: Option<DemuxerParams>,
}

#[derive(Debug, Clone, Default)]
pub struct DemuxerParams {
    pub track_numbers: Option<Vec<u64>>,
}

impl MkvDemuxer {
    pub fn new() -> MkvDemuxer {
        MkvDemuxer {
            header: None,
            seek_head: None,
            info: None,
            tracks: None,
            queue: VecDeque::new(),
            blockstream: Vec::new(),
            params: None,
        }
    }

    pub fn with_params(params: DemuxerParams) -> MkvDemuxer {
        MkvDemuxer {
            params: Some(params),
            ..Default::default()
        }
    }

    pub fn parse_until_tracks<'a>(
        &mut self,
        original_input: &'a [u8],
    ) -> IResult<&'a [u8], (), ebml::Error<'a>> {
        let (i1, header) = ebml_header(original_input)?;

        self.header = Some(header);

        let (mut input, _) = segment(i1)?;

        self.seek_head = None;
        self.info = None;
        self.tracks = None;

        loop {
            if self.seek_head.is_some() && self.info.is_some() && self.tracks.is_some() {
                return Ok((input, ()));
            }

            let (i3, element) = segment_element(input)?;

            match element {
                SegmentElement::SeekHead(s) => {
                    trace!("got seek head: {:#?}", s);
                    self.seek_head = if self.seek_head.is_none() {
                        Some(s)
                    } else {
                        return Err(Err::Error(custom_error(input, 1)));
                    };
                }
                SegmentElement::Info(i) => {
                    trace!("got info: {:#?}", i);
                    self.info = if self.info.is_none() {
                        Some(i)
                    } else {
                        return Err(Err::Error(custom_error(input, 1)));
                    };
                }
                SegmentElement::Tracks(t) => {
                    trace!("got tracks: {:#?}", t);
                    self.tracks = if self.tracks.is_none() {
                        // Only keep tracks we're interested in
                        if let Some(params) = &self.params {
                            if let Some(track_numbers) = &params.track_numbers {
                                let mut t = t;
                                t.tracks = t
                                    .tracks
                                    .into_iter()
                                    .filter(|tr| track_numbers.contains(&tr.track_number))
                                    .collect();
                                Some(t)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        return Err(Err::Error(custom_error(input, 1)));
                    };
                }
                el => {
                    debug!("got element: {:#?}", el);
                }
            }

            input = i3;
        }
    }
}

impl Demuxer for MkvDemuxer {
    fn read_headers(&mut self, buf: &Box<dyn Buffered>, info: &mut GlobalInfo) -> Result<SeekFrom> {
        match self.parse_until_tracks(buf.data()) {
            Ok((i, _)) => {
                info.duration = self
                    .info
                    .as_ref()
                    .and_then(|info| info.duration)
                    .map(|d| d as u64);
                if let Some(ref mut t) = self.tracks {
                    for tr in t.tracks.iter_mut() {
                        info.add_stream(track_to_stream(self.info.as_ref().unwrap(), tr));
                        tr.stream_index = info.streams.last().unwrap().index;
                    }
                }
                Ok(SeekFrom::Current(buf.data().offset(i) as i64))
            }
            Err(Err::Incomplete(needed)) => {
                let sz = match needed {
                    Needed::Size(size) => buf.data().len() + usize::from(size),
                    _ => 1024,
                };
                Err(Error::MoreDataNeeded(sz))
            }
            e => {
                error!("{:?}", e);
                Err(Error::InvalidData)
            }
        }
    }

    fn read_event(&mut self, buf: &Box<dyn Buffered>) -> Result<(SeekFrom, Event)> {
        if let Some(event) = self.queue.pop_front() {
            Ok((SeekFrom::Current(0), event))
        } else {
            match segment_element(buf.data()) {
                Ok((i, element)) => {
                    let seek = SeekFrom::Current(buf.data().offset(i) as i64);
                    match element {
                        SegmentElement::Cluster(c) => {
                            //self.clusters.push(c);
                            debug!("got cluster element at timecode: {}", c.timecode);
                            let mut packets = c.generate_packets(self.tracks.as_ref().unwrap());
                            self.queue.extend(packets.drain(..));
                            if let Some(event) = self.queue.pop_front() {
                                return Ok((seek, event));
                            }
                        }
                        _el => {}
                    }
                    Ok((seek, Event::MoreDataNeeded(0)))
                }
                Err(Err::Incomplete(Needed::Size(size))) => {
                    Err(Error::MoreDataNeeded(usize::from(size)))
                }
                e => {
                    error!("{:?}", e);
                    Err(Error::InvalidData)
                }
            }
        }
    }
}

fn track_entry_codec_id(t: &TrackEntry) -> Option<String> {
    // TODO: Support V_QUICKTIME and V_MS/VFW/FOURCC
    match t.codec_id.as_ref() {
        "A_OPUS" => Some("opus".to_owned()),
        "A_VORBIS" => Some("vorbis".to_owned()),
        "V_AV1" => Some("av1".to_owned()),
        "V_VP8" => Some("vp8".to_owned()),
        "V_VP9" => Some("vp9".to_owned()),
        _ => None,
    }
}

fn track_entry_video_kind(t: &TrackEntry) -> Option<MediaKind> {
    // TODO: Validate that a track::video exists for track::type video before.
    if let Some(ref video) = t.video {
        let v = VideoInfo {
            width: video.pixel_width as usize,
            height: video.pixel_height as usize,
            // TODO parse Colour and/or CodecPrivate to extract the format
            format: None,
        };
        Some(MediaKind::Video(v))
    } else {
        None
    }
}

fn track_entry_audio_kind(t: &TrackEntry) -> Option<MediaKind> {
    use av_data::audiosample::*;
    // TODO: Validate that a track::video exists for track::type video before.
    if let Some(ref audio) = t.audio {
        let rate = if let Some(r) = audio.output_sampling_frequency {
            r
        } else {
            audio.sampling_frequency
        };
        // TODO: complete it
        let map = if audio.channel_positions.is_none() {
            Some(ChannelMap::default_map(audio.channels as usize))
        } else {
            unimplemented!("Convert matroska map to rust-av map")
        };
        let a = AudioInfo {
            rate: rate as usize,
            map,
            format: None,
        };
        Some(MediaKind::Audio(a))
    } else {
        None
    }
}

fn track_entry_media_kind(t: &TrackEntry) -> Option<MediaKind> {
    // TODO: Use an enum for the track type
    match t.track_type {
        0x1 => track_entry_video_kind(t),
        0x2 => track_entry_audio_kind(t),
        _ => None,
    }
}

// TODO: make sure the timecode_scale isn't 0
pub fn track_to_stream(info: &Info, t: &TrackEntry) -> Stream {
    let num = if let Some(ts) = t.track_timecode_scale {
        (ts * info.timecode_scale as f64) as i64
    } else {
        info.timecode_scale as i64
    };

    Stream {
        id: t.track_uid as isize,
        index: t.stream_index,
        start: None,
        duration: t.default_duration,
        timebase: Rational64::new(num, 1000 * 1000 * 1000),
        // TODO: Extend CodecParams and fill it with the remaining information
        params: CodecParams {
            extradata: t.codec_private.clone(),
            bit_rate: 0,
            delay: t.codec_delay.unwrap_or(0) as usize,
            convergence_window: t.seek_pre_roll.unwrap_or(0) as usize,
            codec_id: track_entry_codec_id(t),
            kind: track_entry_media_kind(t),
        },
        user_private: None,
    }
}

impl<'a> Cluster<'a> {
    pub fn generate_packets(&self, tracks: &Tracks) -> Vec<Event> {
        let mut v = Vec::new();

        for block_data in self.simple_block.iter() {
            if let Ok((i, block)) = simple_block(block_data) {
                debug!("parsing simple block: {:?}", block);
                if let Some(index) = tracks.lookup(block.track_number) {
                    let packet = Packet {
                        data: i.into(),
                        t: TimeInfo {
                            pts: Some(i64::from(block.timecode)),
                            dts: None,
                            duration: None,
                            timebase: None,
                            user_private: None,
                        },
                        pos: None,
                        stream_index: index as isize,
                        is_key: block.keyframe,
                        is_corrupted: false,
                    };

                    v.push(Event::NewPacket(packet));
                }
            } else {
                error!("error parsing simple block");
            }
        }

        v
    }
}

struct Des {
    d: Descr,
}

impl Descriptor for Des {
    fn create(&self) -> Box<dyn Demuxer> {
        Box::new(MkvDemuxer::new())
    }
    fn describe(&self) -> &Descr {
        &self.d
    }
    fn probe(&self, data: &[u8]) -> u8 {
        match ebml_header(&data[..100]) {
            Ok(_) => 100,
            _ => 0,
        }
    }
}

pub const MKV_DESC: &dyn Descriptor = &Des {
    d: Descr {
        name: "matroska",
        demuxer: "mkv",
        description: "Nom-based Matroska demuxer",
        extensions: &["mkv", "webm", "mka"],
        mime: &["video/x-matroska", "audio/x-matroska"],
    },
};

#[cfg(test)]
#[allow(non_upper_case_globals)]
mod tests {
    use std::io::Cursor;

    use log::info;
    use nom::Offset;

    use av_format::{buffer::*, demuxer::Context};

    use super::*;

    const webm: &[u8] = include_bytes!("../assets/bbb-vp9-opus.webm");

    #[test]
    fn parse_headers() {
        let mut demuxer = MkvDemuxer::new();

        let res = demuxer.parse_until_tracks(webm);
        info!("got parsing res: {:?}", res);
        match res {
            Ok((i, _)) => {
                info!("offset: {} bytes", webm.offset(i));
            }
            e => {
                info!("could not parse: {:?}", e);
            }
        }

        info!("demuxer: {:#?}", demuxer);
    }

    #[test]
    fn parse_headers_incremental_buffer() {
        let mut demuxer = MkvDemuxer::new();

        for n in 100..2000 {
            let res = demuxer.parse_until_tracks(&webm[0..n]);
            match res {
                Ok(_) => info!("Size {} ok", n),
                Err(Err::Incomplete(needed)) => info!("Incomplete {} needs {:?}", n, needed),
                Err(e) => {
                    panic!("Error at size {}: {:?}", n, e);
                }
            }
        }
    }

    #[test]
    fn context() {
        let mut context = Context::new(
            Box::new(MkvDemuxer::new()),
            Box::new(AccReader::new(Cursor::new(webm))),
        );
        info!("read headers: {:?}", context.read_headers().unwrap());
        info!("streams: {:?}", context.info.streams);
        info!("event: {:?}", context.read_event().unwrap());
        info!("event: {:?}", context.read_event().unwrap());
        info!("event: {:?}", context.read_event().unwrap());
        info!("event: {:?}", context.read_event().unwrap());
        info!("event: {:?}", context.read_event().unwrap());
        info!("event: {:?}", context.read_event().unwrap());
        info!("event: {:?}", context.read_event().unwrap());
        info!("event: {:?}", context.read_event().unwrap());
    }
}
