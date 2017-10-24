use av_format::error::*;
use std::io::SeekFrom;
use av_data::packet::Packet;
use av_format::stream::*;
use av_format::buffer::Buffered;
use av_format::demuxer::demux::{Demuxer, Event};
use av_format::demuxer::context::GlobalInfo;
use std::collections::VecDeque;
use rational::Rational32;

use ebml::{ebml_header, EBMLHeader};
use elements::{segment, segment_element, Cluster, SeekHead, Info, Tracks, TrackEntry,
               SegmentElement, simple_block};
use nom::{self, Err, IResult, Offset};

#[derive(Debug, Clone, PartialEq)]
pub struct MkvDemuxer {
    pub header: Option<EBMLHeader>,
    pub seek_head: Option<SeekHead>,
    pub info: Option<Info>,
    pub tracks: Option<Tracks>,
    pub queue: VecDeque<Event>,
    pub blockstream: Vec<u8>,
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
        }
    }

    pub fn parse_until_tracks<'a>(&mut self, original_input: &'a [u8]) -> IResult<&'a [u8], ()> {
        let (i1, header) = try_parse!(original_input, ebml_header);

        self.header = Some(header);

        let (mut input, _) = try_parse!(i1, segment);

        loop {
            if self.seek_head.is_some() && self.info.is_some() && self.tracks.is_some() {
                return Ok((input, ()));
            }

            println!("offset: {}", original_input.offset(input));

            let (i3, element) = try_parse!(input, segment_element);

            match element {
                SegmentElement::SeekHead(s) => {
                    println!("got seek head: {:#?}", s);
                    if self.seek_head.is_some() {
                        return Err(Err::Error(error_position!(nom::ErrorKind::Custom(1), input)));
                    } else {
                        self.seek_head = Some(s);
                    }
                }
                SegmentElement::Info(i) => {
                    println!("got info: {:#?}", i);
                    if self.info.is_some() {
                        return Err(Err::Error(error_position!(nom::ErrorKind::Custom(1), input)));
                    } else {
                        self.info = Some(i);
                    }
                }
                SegmentElement::Tracks(t) => {
                    println!("got tracks: {:#?}", t);
                    if self.tracks.is_some() {
                        return Err(Err::Error(error_position!(nom::ErrorKind::Custom(1), input)));
                    } else {
                        self.tracks = Some(t);
                    }
                }
                SegmentElement::Cluster(c) => {
                    println!("got a cluster: {:#?}", c);
                    //self.clusters.push(c);
                }
                el => {
                    println!("got element: {:#?}", el);
                }
            }

            input = i3;
        }
    }
}

impl Demuxer for MkvDemuxer {
    fn open(&mut self) {}

    fn read_headers(&mut self, buf: &Box<Buffered>, info: &mut GlobalInfo) -> Result<SeekFrom> {
        match self.parse_until_tracks(buf.data()) {
            Ok((i, _)) => {
                info.duration = self.info.as_ref().and_then(|info| info.duration).map(|d| d as u64);
                if let Some(ref t) = self.tracks {
                    info.streams = t.tracks.iter().map(|tr| track_to_stream(tr)).collect();
                }
                Ok(SeekFrom::Current(buf.data().offset(i) as i64))
            }
             Err(Err::Incomplete(_)) => Err(ErrorKind::MoreDataNeeded.into()),
            e => {
                println!("error reading headers: {:?}", e);
                Err(ErrorKind::InvalidData.into())
            }
        }
    }

    fn read_packet(&mut self, buf: &Box<Buffered>) -> Result<(SeekFrom, Event)> {
        if let Some(event) = self.queue.pop_front() {
            Ok((SeekFrom::Current(0), event))
        } else {
            println!("no more stored packet, parsing a new one");
            match segment_element(buf.data()) {
                Ok((i, element)) => {
                    let seek = SeekFrom::Current(buf.data().offset(i) as i64);
                    match element {
                        SegmentElement::Cluster(c) => {
                            //self.clusters.push(c);
                            println!("got cluster element at timecode: {}", c.timecode);
                            let mut packets = c.generate_packets();
                            self.queue.extend(packets.drain(..));
                            /*
                        for block in c.simple_block.iter() {
                          println!("got simple block of size {}", block.len());
                          if let IResult::Done(_,o) = simple_block(block) {
                            println!("parsing simple block: {:?}", o);
                          } else {
                            println!("error parsing simple block");
                          }
                          self.blockstream.extend(*block);
                        }

                        for block_group in c.block_group.iter() {
                          println!("got block group of size {}", block_group.block.len());
                          self.blockstream.extend(block_group.block);
                        }
                        */
                            if let Some(event) = self.queue.pop_front() {
                                return Ok((seek, event));
                            }
                        }
                        el => {
                            println!("got element: {:#?}", el);
                        }
                    }

                    Ok((seek, Event::MoreDataNeeded))
                }
                 Err(Err::Incomplete(_)) => Ok((SeekFrom::Current(0), Event::MoreDataNeeded)),
                e => {
                    println!("parsing issue: {:?}", e);
                    Err(ErrorKind::InvalidData.into())
                }
            }
        }
    }
}

fn track_entry_codec_id(t: &TrackEntry) -> Option<CodecID> {
    // TODO: Support V_QUICKTIME and V_MS/VFW/FOURCC
    match t.codec_id.as_ref() {
        "A_OPUS" => Some(CodecID::Opus),
        "V_VP9" => Some(CodecID::VP9),
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
    // TODO: Validate that a track::video exists for track::type video before.
    if let Some(ref audio) = t.audio {
        let rate = if let Some(r) = audio.output_sampling_frequency {
            r
        } else {
            audio.sampling_frequency
        };
        // TODO: complete it
        let a = AudioInfo {
            samples: 0,
            rate: rate as usize,
            map: None,
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

pub fn track_to_stream(t: &TrackEntry) -> Stream {
    Stream {
        id: t.track_uid as usize,
        index: t.track_number as usize,
        start: None,
        duration: t.default_duration,
        timebase: Rational32::from_integer(1),
        // TODO: Extend CodecParams and fill it with the remaining information
        params: CodecParams {
            extradata: t.codec_private.clone(),
            bit_rate: 0,
            codec_id: track_entry_codec_id(t),
            kind: track_entry_media_kind(t),
        },
    }
}

impl<'a> Cluster<'a> {
    pub fn generate_packets(&self) -> Vec<Event> {
        let mut v = Vec::new();

        for block_data in self.simple_block.iter() {
            if let Ok((i, block)) = simple_block(block_data) {
                //println!("parsing simple block: {:?}", block);
                let packet = Packet {
                    data: i.into(),
                    pts: Some(block.timecode as i64),
                    dts: None,
                    pos: None,
                    stream_index: block.track_number as isize,
                    is_key: block.keyframe,
                    is_corrupted: false,
                };

                v.push(Event::NewPacket(packet));
            } else {
                println!("error parsing simple block");
            }
        }

        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use nom::Offset;
    use av_format::demuxer::context::DemuxerContext;

    const webm: &'static [u8] = include_bytes!("../assets/bbb-vp9-opus.webm");

    #[test]
    fn parse_headers() {
        let mut demuxer = MkvDemuxer::new();

        let res = demuxer.parse_until_tracks(webm);
        println!("got parsing res: {:?}", res);
        match res {
            Ok((i, _)) => {
                println!("offset: {} bytes", webm.offset(i));
            }
            e => {
                println!("could not parse: {:?}", e);
            }
        }

        println!("demuxer: {:#?}", demuxer);
        panic!();
    }

    #[test]
    fn context() {
        let mut context = DemuxerContext::new(Box::new(MkvDemuxer::new()),
                                              Box::new(Cursor::new(webm)));
        println!("DEMUXER CONTEXT read headers: {:?}", context.read_headers());
        println!("DEMUXER CONTEXT streams: {:?}", context.info.streams);
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        panic!();
    }
}
