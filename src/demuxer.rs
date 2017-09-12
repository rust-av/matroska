use av_format::*;
use std::io::{Error,ErrorKind,SeekFrom};
use av_data::packet::Packet;
use av_format::buffer::Buffered;
use av_format::demuxer::demux::Demuxer;

use ebml::{ebml_header, EBMLHeader};
use nom::{IResult,Offset};

pub struct MkvDemuxer {
  pub header: Option<EBMLHeader>,

}

impl Demuxer for MkvDemuxer {
  fn open(&mut self) {}

  fn read_headers(&mut self, ctx: &Box<Buffered>) -> Result<SeekFrom, Error> {
    match ebml_header(ctx.data()) {
      IResult::Done(i, o) => {
        self.header = Some(o);
        Ok(SeekFrom::Current(ctx.data().offset(i) as i64))
      },
      _ => {
        Err(Error::new(ErrorKind::Other, "MKV header parsing error"))
      }
    }
  }

  fn read_packet(&mut self, ctx: &Box<Buffered>) -> Result<(SeekFrom,Packet), Error> {
    Err(Error::new(ErrorKind::Other, "MKV packet parsing error"))
  }
}
