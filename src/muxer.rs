use std::sync::Arc;
use av_format::error::*;
use av_format::muxer::*;
use av_format::common::GlobalInfo;
use av_format::stream::Stream;
use av_data::value::Value;
use av_data::packet::Packet;
use av_data::params::{MediaKind};


use ebml::EBMLHeader;
use elements::{SeekHead,Info,Tracks,Cluster,TrackEntry, Audio, Video};
use serializer::ebml::gen_ebml_header;
use serializer::elements::{gen_segment_header, gen_seek_head, gen_info, gen_tracks};
use cookie_factory::GenError;

use nom::HexDisplay;

#[derive(Debug, Clone, PartialEq)]
pub struct MkvMuxer {
  header:    EBMLHeader,
  seek_head: SeekHead,
  info:      Option<Info>,
  tracks:    Option<Tracks>,
}

impl MkvMuxer {
  pub fn Matroska() -> MkvMuxer {
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
        positions: Vec::new()
      },
      info: None,
      tracks: None
    }
  }

  pub fn Webm() -> MkvMuxer {
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
        positions: Vec::new()
      },
      info: None,
      tracks: None
    }
  }

  pub fn write_ebml_header(&mut self, buf: &mut Vec<u8>) -> Result<()> {
      let mut origin = (&buf).as_ptr() as usize;

      let mut needed = 0usize;
      let mut offset = 0usize;
      loop {
        if needed > 0 {
          let len = needed + buf.len();
          buf.resize(len, 0);
          needed = 0;
          origin = (&buf).as_ptr() as usize;
        }

        match gen_ebml_header((buf, 0), &self.header) {
          Err(GenError::BufferTooSmall(sz)) => {
            needed = sz;
          },
          Err(e) => {
            println!("muxing error: {:?}", e);
            return Err(Error::InvalidData);
          },
          Ok((sl, sz)) => {
            offset = sl.as_ptr() as usize + sz - origin;
            break;
          }
        };
      }
      buf.truncate(offset);

      Ok(())
  }

  pub fn write_segment_header(&mut self, buf: &mut Vec<u8>, size: usize) -> Result<()> {
      let mut origin = (&buf).as_ptr() as usize;

      let mut needed = 0usize;
      let mut offset = 0usize;
      loop {
        if needed > 0 {
          let len = needed + buf.len();
          buf.resize(len, 0);
          needed = 0;
          origin = (&buf).as_ptr() as usize;
        }

        match gen_segment_header((buf, 0), size as u64) {
          Err(GenError::BufferTooSmall(sz)) => {
            needed = sz;
          },
          Err(e) => {
            println!("muxing error: {:?}", e);
            return Err(Error::InvalidData);
          },
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
      let mut origin = (&buf).as_ptr() as usize;

      let mut needed = 0usize;
      let mut offset = 0usize;
      loop {
        if needed > 0 {
          let len = needed + buf.len();
          buf.resize(len, 0);
          needed = 0;
          origin = (&buf).as_ptr() as usize;
        }

        match gen_seek_head((buf, 0), &self.seek_head) {
          Err(GenError::BufferTooSmall(sz)) => {
            needed = sz;
          },
          Err(e) => {
            println!("muxing error: {:?}", e);
            return Err(Error::InvalidData);
          },
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
    if let Some(ref info) = self.info.as_ref() {

      let mut origin = (&buf).as_ptr() as usize;
      let mut needed = 0usize;
      let mut offset = 0usize;
      loop {
        if needed > 0 {
          let len = needed + buf.len();
          buf.resize(len, 0);
          needed = 0;
          origin = (&buf).as_ptr() as usize;
        }

        match gen_info((buf, 0), info) {
          Err(GenError::BufferTooSmall(sz)) => {
            needed = sz;
          },
          Err(e) => {
            println!("muxing error: {:?}", e);
            return Err(Error::InvalidData);
          },
          Ok((sl, sz)) => {
            offset = sl.as_ptr() as usize + sz - origin;
            break;
          }
        };
      }
      println!("info: offset {:?}", offset);
      buf.truncate(offset);
    }
    Ok(())
  }

  pub fn write_tracks(&mut self, buf: &mut Vec<u8>) -> Result<()> {
    if let Some(ref tracks) = self.tracks.as_ref() {

      let mut origin = (&buf).as_ptr() as usize;
      let mut needed = 0usize;
      let mut offset = 0usize;
      loop {
        if needed > 0 {
          let len = needed + buf.len();
          buf.resize(len, 0);
          needed = 0;
          origin = (&buf).as_ptr() as usize;
        }

        match gen_tracks((buf, 0), tracks) {
          Err(GenError::BufferTooSmall(sz)) => {
            needed = sz;
          },
          Err(e) => {
            println!("tracks muxing error: {:?}", e);
            return Err(Error::InvalidData);
          },
          Ok((sl, sz)) => {
            offset = sl.as_ptr() as usize + sz - origin;
            break;
          }
        };
      }
      println!("info: offset {:?}", offset);
      buf.truncate(offset);
    }
    Ok(())
  }
}

impl Muxer for MkvMuxer {
    fn configure(&mut self) -> Result<()> {
      Ok(())
    }

    fn write_header(&mut self, buf: &mut Vec<u8>) -> Result<()> {
      let mut ebml_header = Vec::new();
      self.write_ebml_header(&mut ebml_header)?;
      let mut seek_head = Vec::new();
      self.write_seek_head(&mut seek_head)?;
      let mut info = Vec::new();
      self.write_info(&mut info)?;
      let mut tracks = Vec::new();
      self.write_tracks(&mut tracks)?;
      println!("tracks:\n{}", (&tracks[..]).to_hex(16));

      let size = ebml_header.len() + seek_head.len() + info.len() + tracks.len();
      println!("segment size: {}", size);
      let mut segment_header = Vec::new();
      self.write_segment_header(&mut segment_header, size)?;

      buf.extend_from_slice(&ebml_header);
      buf.extend_from_slice(&segment_header);
      buf.extend_from_slice(&seek_head);
      buf.extend_from_slice(&info);
      buf.extend_from_slice(&tracks);

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
      println!("info.streams: {:#?}", info.streams);
      self.tracks = Some(Tracks {
        tracks: info.streams.iter().map(stream_to_track).collect()
      });

      let info = Info {
        muxing_app: String::from("rust-av"),
        writing_app: String::from("rust-av"),
        duration: info.duration.map(|d| d as f64),
        ..Default::default()
      };

      self.info = Some(info);
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

pub fn stream_to_track(s: &Stream) -> TrackEntry {
  /*let track_type = match s.params.kind {
    Some(MediaKind::Video(_)) => 0x1,
    Some(MediaKind::Audio(_)) => 0x2,
    _                         => 0,
  };
  */

  let codec_id = match s.params.codec_id.as_ref().map(|s| s.as_str()) {
    Some("opus") => String::from("A_OPUS"),
    Some("vp9")  => String::from("V_VP9"),
    _            => String::from("INVALID_CODEC"),
  };


  let mut t = TrackEntry {
    track_uid: s.id as u64,
    track_number: s.index as u64,
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
      t.track_type = 0x1;
      /*t.video = Some(Video {
        pixel_width:  v.width as u64,
        pixel_height: v.height as u64,
        ..Default::default()
      });*/
    },
    Some(MediaKind::Audio(ref a)) => {
      t.track_type = 0x2;
      t.audio = Some(Audio {
        sampling_frequency: a.rate as f64,
        channels: 1,
        ..Default::default()
      });
    },
    _ => {}
  }

  println!("genetrated track entry:\nuid:{}, number:{}, type:{}", t.track_uid, t.track_number, t.track_type);


  t
}
