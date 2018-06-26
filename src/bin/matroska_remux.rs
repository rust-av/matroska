extern crate matroska;
extern crate av_format;
extern crate av_data;
#[macro_use] extern crate nom;

use av_format::error::*;
use std::io::SeekFrom;
use av_data::packet::Packet;
use av_data::timeinfo::TimeInfo;
use av_data::params::*;
use av_data::rational::Rational64;
use av_format::stream::Stream;
use av_format::buffer::{AccReader, Buffered};
use av_format::demuxer::{self, Demuxer, Event};
use av_format::demuxer::{Descr, Descriptor};
use av_format::muxer::{self};
use av_format::common::GlobalInfo;
use std::collections::VecDeque;
use std::io::{Cursor, Seek};
use std::sync::Arc;
use std::fs::File;

use matroska::ebml::{ebml_header, EBMLHeader};
use matroska::elements::{segment, segment_element, Cluster, SeekHead, Info, Tracks, TrackEntry,
               SegmentElement, simple_block};
use matroska::demuxer::{MkvDemuxer, MKV_DESC};
use matroska::muxer::MkvMuxer;

fn main() {
  const webm: &'static [u8] = include_bytes!("../../assets/bbb-vp9-opus.webm");
  let d = MKV_DESC.create();
  let c = Cursor::new(webm);
  let acc = AccReader::with_capacity(4096, c);
  let input = Box::new(acc);
  let mut demuxer = demuxer::Context::new(d, input);

  println!("read headers: {:?}", demuxer.read_headers().unwrap());
  println!("global info: {:#?}", demuxer.info);

  let mux = Box::new(MkvMuxer::Webm());
  //let mut output:Vec<u8> = Vec::with_capacity(24000);
  let mut output = File::create("output.webm").unwrap();

  let mut muxer = muxer::Context::new(mux, Box::new(output));
  muxer.configure().unwrap();
  muxer.set_global_info(demuxer.info.clone()).unwrap();
  muxer.write_header().unwrap();

  /*
  loop {
    let event = demuxer.read_event().unwrap();
    //println!("event: {:?}", event);
    match event {
      Event::MoreDataNeeded(sz) => panic!("we needed more data"),
      Event::NewStream(s) => panic!("new stream :{:?}", s),
      Event::NewPacket(packet) => {
        println!("writing packet");
        muxer.write_packet(Arc::new(packet)).unwrap();
      }
    }
  }
  */


  /*
  let mut demuxer = MkvDemuxer::new();

  let mut global_info = GlobalInfo {
    duration: None,
    timebase: None,
    streams:  Vec::new(),
  };


  let seek = demuxer.read_headers(&input, &mut global_info).unwrap();
  input.seek(seek).unwrap();
  */



}

/*
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

        self.seek_head = None;
        self.info = None;
        self.tracks = None;

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

use nom::Needed;

impl Demuxer for MkvDemuxer {
    fn read_headers(&mut self, buf: &Box<Buffered>, info: &mut GlobalInfo) -> Result<SeekFrom> {
        match self.parse_until_tracks(buf.data()) {
            Ok((i, _)) => {
                info.duration = self.info.as_ref().and_then(|info| info.duration).map(|d| d as u64);
                if let Some(ref t) = self.tracks {
                    info.streams = t.tracks.iter().map(|tr| {
                        track_to_stream(self.info.as_ref().unwrap(), tr)
                    }).collect();
                }
                Ok(SeekFrom::Current(buf.data().offset(i) as i64))
            },
            Err(Err::Incomplete(needed)) => {
                let sz = match needed {
                    Needed::Size(size) => size,
                    _ => 1024,
                };
                Err(Error::MoreDataNeeded(sz).into())
            },
            e => {
                println!("error reading headers: {:?}", e);
                Err(Error::InvalidData.into())
            }
        }
    }

    fn read_event(&mut self, buf: &Box<Buffered>) -> Result<(SeekFrom, Event)> {
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
                            if let Some(event) = self.queue.pop_front() {
                                return Ok((seek, event));
                            }
                        }
                        el => {
                            println!("got element: {:#?}", el);
                        }
                    }
                    Ok((seek, Event::MoreDataNeeded(0)))
                },
                Err(Err::Incomplete(Needed::Size(size))) => {
                    Err(Error::MoreDataNeeded(size).into())
                },
                e => {
                    println!("parsing issue: {:?}", e);
                    Err(Error::InvalidData.into())
                }
            }
        }
    }
}

fn track_entry_codec_id(t: &TrackEntry) -> Option<String> {
    // TODO: Support V_QUICKTIME and V_MS/VFW/FOURCC
    match t.codec_id.as_ref() {
        "A_OPUS" => Some("opus".to_owned()),
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
    // TODO: Validate that a track::video exists for track::type video before.
    if let Some(ref audio) = t.audio {
        let rate = if let Some(r) = audio.output_sampling_frequency {
            r
        } else {
            audio.sampling_frequency
        };
        // TODO: complete it
        let a = AudioInfo {
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

// TODO: make sure the timecode_scale isn't 0
pub fn track_to_stream(info: &Info, t: &TrackEntry) -> Stream {
    let num = if let Some(ts) = t.track_timecode_scale  {
        (ts * info.timecode_scale as f64) as i64
    } else {
        info.timecode_scale as i64
    };

    Stream {
        id: t.track_uid as usize,
        index: t.track_number as usize,
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
    }
}
*/

/*
impl<'a> Cluster<'a> {
    pub fn generate_packets(&self) -> Vec<Event> {
        let mut v = Vec::new();

        for block_data in self.simple_block.iter() {
            if let Ok((i, block)) = simple_block(block_data) {
                //println!("parsing simple block: {:?}", block);
                let packet = Packet {
                    data: i.into(),
                    t: TimeInfo {
                        pts: Some(block.timecode as i64),
                        dts: None,
                        duration: None,
                        timebase: None,
                    },
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

struct Des {
    d: Descr,
}

impl Descriptor for Des {
    fn create(&self) -> Box<Demuxer> {
        Box::new(MkvDemuxer::new())
    }
    fn describe<'a>(&'a self) -> &'a Descr {
        &self.d
    }
    fn probe(&self, data: &[u8]) -> u8 {
        match ebml_header(&data[..100]) {
            Ok(_) => 100,
            _ => 0,
        }
    }
}

pub const MKV_DESC: &Descriptor = &Des {
    d: Descr {
        name: "matroska",
        demuxer: "mkv",
        description: "Nom-based Matroska demuxer",
        extensions: &["mkv", "webm", "mka"],
        mime: &["video/x-matroska", "audio/x-matroska"],
    }
};

*/
