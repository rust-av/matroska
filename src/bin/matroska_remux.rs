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
    match demuxer.read_event() {
      Ok(event) => {
        //println!("event: {:?}", event);
        match event {
          Event::MoreDataNeeded(sz) => panic!("we needed more data"),
          Event::NewStream(s) => panic!("new stream :{:?}", s),
          Event::NewPacket(packet) => {
            //println!("writing packet");
            muxer.write_packet(Arc::new(packet)).unwrap();
          }
        }
      },
      Err(e) => {
        println!("error: {:?}", e);
        muxer.write_trailer();
        break;
      }
    }
  }
  */
}

