use std::sync::Arc;
use av_format::error::*;
use av_format::muxer::*;
use av_format::common::GlobalInfo;
use av_data::value::Value;
use av_data::packet::Packet;


use ebml::EBMLHeader;
use elements::{SeekHead,Info,Tracks,Cluster};
use serializer::ebml::gen_ebml_header;
use serializer::elements::{gen_seek_head, gen_info, gen_tracks};
use cookie_factory::GenError;

#[derive(Debug, Clone, PartialEq)]
pub struct MkvMuxer {
  header:    EBMLHeader,
  seek_head: SeekHead,
  info:      Info,
  tracks:    Tracks,
}

impl MkvMuxer {
  pub fn new(header: EBMLHeader, seek_head: SeekHead, info: Info, tracks: Tracks) -> MkvMuxer {
    MkvMuxer {
      header, seek_head, info, tracks
    }
  }
}

impl Muxer for MkvMuxer {
    fn configure(&mut self) -> Result<()> {
      Ok(())
    }

    fn write_header(&mut self, buf: &mut Vec<u8>) -> Result<()> {
      let origin = (&buf).as_ptr() as usize;

      let offset: usize = match gen_ebml_header((buf, 0), &self.header) {
        Err(GenError::BufferTooSmall(sz)) => {
          return Err(Error::MoreDataNeeded(sz));
        },
        Err(e) => {
          println!("muxing error: {:?}", e);
          return Err(Error::InvalidData);
        },
        Ok((sl, sz)) => {
          sl.as_ptr() as usize + sz - origin
        }
      };

      buf.truncate(offset);
      Ok(())
    }

    fn write_packet(&mut self, buf: &mut Vec<u8>, pkt: Arc<Packet>) -> Result<()> {
      let origin = (&buf).as_ptr() as usize;

      let cluster = Cluster {
        timecode: 0,
        silent_tracks: None,
        position: None,
        prev_size: None,
        simple_block: Vec::new(),
        block_group: Vec::new(),
        encrypted_block: None,
      };

      Ok(())
    }

    fn write_trailer(&mut self, buf: &mut Vec<u8>) -> Result<()> {
      Ok(())
    }

    fn set_global_info(&mut self, info: GlobalInfo) -> Result<()> {
      Ok(())
    }

    fn set_option<'a>(&mut self, key: &str, val: Value<'a>) -> Result<()> {
      Ok(())
    }
}

fn offset<'a>(original: &(&'a[u8], usize), subslice: &(&'a[u8], usize)) -> usize {
  let first = original.0.as_ptr() as usize;
  let second = subslice.0.as_ptr() as usize;

  second + subslice.1 - first - original.1
}

pub fn gen_mkv_prefix<'a>(input: (&'a mut [u8], usize), header: &EBMLHeader, seek_head: &SeekHead, info: &Info, tracks: &Tracks)
  -> ::std::result::Result<(&'a mut [u8], usize), GenError> {

  do_gen!(input,
    gen_call!(gen_ebml_header, header)  >>
    gen_call!(gen_seek_head, seek_head) >>
    gen_call!(gen_info, info)           >>
    gen_call!(gen_tracks, tracks)
  )
}
