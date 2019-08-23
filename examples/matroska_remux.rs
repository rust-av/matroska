#[macro_use] extern crate log;

use av_format::{
    buffer::AccReader,
    demuxer::{self, Event},
    muxer,
};
use matroska::{demuxer::MKV_DESC, muxer::MkvMuxer};
use std::{fs::File, io::{BufRead, Cursor}, sync::Arc};

fn main() {
    pretty_env_logger::init();
    let mut args = std::env::args();
    let _ = args.next();
    let (input, output) = (args.next(), args.next());

    if input.is_none() || output.is_none() {
      println!("usage: matroska_remux input.mkv output.mkv");
      return;
    }

    let (input_path, output_path) = (input.unwrap(), output.unwrap());

    //const webm: &'static [u8] = include_bytes!("../assets/big-buck-bunny_trailer.webm");
    //const WEBM: &'static [u8] = include_bytes!("../assets/bbb-vp9-opus.webm");
    //const webm: &'static [u8] = include_bytes!("../assets/single_stream.mkv");
    //const WEBM: &'static [u8] = include_bytes!("../assets/single_stream_av1.mkv");
    //let c = Cursor::new(WEBM);

    let file = std::fs::File::open(input_path).unwrap();
    //let acc = AccReader::with_capacity(5242880, file);
    let mut acc = AccReader::with_capacity(20000, file);
    let input = Box::new(acc);

    let d = MKV_DESC.create();
    let mut demuxer = demuxer::Context::new(d, input);

    debug!("read headers: {:?}", demuxer.read_headers().unwrap());
    debug!("global info: {:#?}", demuxer.info);

    let mux = Box::new(MkvMuxer::webm());
    let output = File::create(output_path).unwrap();

    let mut muxer = muxer::Context::new(mux, Box::new(output));
    muxer.configure().unwrap();
    muxer.set_global_info(demuxer.info.clone()).unwrap();
    muxer.write_header().unwrap();

    loop {
        match demuxer.read_event() {
            Ok(event) => {
                //println!("event: {:?}", event);
                match event {
                    Event::MoreDataNeeded(sz) => panic!("we needed more data: {} bytes", sz),
                    Event::NewStream(s) => panic!("new stream :{:?}", s),
                    Event::NewPacket(packet) => {
                        //println!("writing packet");
                        muxer.write_packet(Arc::new(packet)).unwrap();
                    }
                    Event::Eof => {
                        muxer.write_trailer().unwrap();
                        break;
                    }
                }
            }
            Err(e) => {
                println!("error: {:?}", e);
                break;
            }
        }
    }
}
