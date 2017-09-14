use av_format::error::*;
use std::io::SeekFrom;
use av_data::packet::Packet;
use av_format::stream::Stream;
use av_format::buffer::Buffered;
use av_format::demuxer::demux::{Demuxer, Event};
use av_format::demuxer::context::GlobalInfo;
use std::collections::VecDeque;
use rational::Rational32;

use ebml::{ebml_header, EBMLHeader};
use elements::{segment, segment_element, Cluster, SeekHead, Info, Tracks, TrackEntry, SegmentElement};
use nom::{self, IResult, Offset};

#[derive(Debug,Clone,PartialEq)]
pub struct MkvDemuxer {
    pub header: Option<EBMLHeader>,
    pub seek_head: Option<SeekHead>,
    pub info: Option<Info>,
    pub tracks: Option<Tracks>,
    pub clusters: Vec<Cluster>,
    pub queue: VecDeque<Event>,
}

impl MkvDemuxer {
    pub fn new() -> MkvDemuxer {
        MkvDemuxer {
            header: None,
            seek_head: None,
            info: None,
            tracks: None,
            clusters: Vec::new(),
            queue: VecDeque::new(),
        }
    }

    pub fn parse_until_tracks<'a>(&mut self, original_input: &'a [u8]) -> IResult<&'a [u8], ()> {
        let (i1, header) = try_parse!(original_input, ebml_header);

        self.header = Some(header);

        let (mut input, _) = try_parse!(i1, segment);

        loop {
            if self.seek_head.is_some() && self.info.is_some() && self.tracks.is_some() {
                return IResult::Done(input, ());
            }

            println!("offset: {}", original_input.offset(input));

            let (i3, element) = try_parse!(input, segment_element);

            match element {
                SegmentElement::SeekHead(s) => {
                    println!("got seek head: {:#?}", s);
                    if self.seek_head.is_some() {
                        return IResult::Error(nom::ErrorKind::Custom(1));
                    } else {
                        self.seek_head = Some(s);
                    }
                }
                SegmentElement::Info(i) => {
                    println!("got info: {:#?}", i);
                    if self.info.is_some() {
                        return IResult::Error(nom::ErrorKind::Custom(1));
                    } else {
                        self.info = Some(i);
                    }
                }
                SegmentElement::Tracks(t) => {
                    println!("got tracks: {:#?}", t);
                    if self.tracks.is_some() {
                        return IResult::Error(nom::ErrorKind::Custom(1));
                    } else {
                        self.tracks = Some(t);
                    }
                }
                SegmentElement::Cluster(c) => {
                    println!("got a cluster: {:#?}", c);
                    self.clusters.push(c);
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
            IResult::Done(i, _) => {
                info.duration = self.info.as_ref().and_then(|info| info.duration).map(|d| d as u64);
                //self.tracks.as_ref().map(|t| t.tracks.
                Ok(SeekFrom::Current(buf.data().offset(i) as i64))
            }
            IResult::Incomplete(_) => Err(ErrorKind::MoreDataNeeded.into()),
            e => Err(ErrorKind::InvalidData.into()),
        }
    }

    fn read_packet(&mut self, buf: &Box<Buffered>) -> Result<(SeekFrom, Event)> {
        match segment_element(buf.data()) {
            IResult::Done(i, element) => {
                let seek = SeekFrom::Current(buf.data().offset(i) as i64);
                match element {
                    SegmentElement::Cluster(c) => {
                        self.clusters.push(c);
                    }
                    el => {
                        println!("got element: {:#?}", el);
                    }
                }

                Ok((seek, Event::MoreDataNeeded))
            }
            IResult::Incomplete(_) => Ok((SeekFrom::Current(0), Event::MoreDataNeeded)),
            e => {
                println!("parsing issue: {:?}", e);
                Err(ErrorKind::InvalidData.into())
            }
        }
    }
}

/*
pub fn track_stream(t: &TrackEntry) -> Stream {

  Stream {
    id: t.track_uid as usize,
    index: t.track_number as usize,
    start: None,
    duration: t.default_duration,
    timebase : Rational32::from_integer(1),
    params : CodecParams {
      extradata: Vec<u8>,
      tag: u32,
      bit_rate: usize,
      bits_per_coded_sample: usize,
      profile: usize,
      level: usize,
      codec_id: CodecID,
      kind: MediaKind::

    },
  }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use nom::{IResult, Offset};
    use av_format::demuxer::context::DemuxerContext;

    const mkv: &'static [u8] = include_bytes!("../assets/single_stream.mkv");

    #[test]
    fn parse_headers() {
        let mut demuxer = MkvDemuxer::new();

        let res = demuxer.parse_until_tracks(mkv);
        println!("got parsing res: {:?}", res);
        match res {
            IResult::Done(i, _) => {
                println!("offset: {} bytes", mkv.offset(i));
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
                                              Box::new(Cursor::new(mkv)));
        println!("DEMUXER CONTEXT read headers: {:?}", context.read_headers());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        println!("DEMUXER CONTEXT event: {:?}", context.read_packet());
        panic!();
    }
}
