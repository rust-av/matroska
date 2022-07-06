use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::{fs::File, sync::Arc};

use clap::Parser;
use log::{debug, error};

use av_format::demuxer::Context as DemuxerCtx;
use av_format::{
    buffer::AccReader,
    demuxer::Event,
    muxer::{self, Writer},
};
use matroska::{demuxer::MkvDemuxer, muxer::MkvMuxer};

#[derive(Parser, Debug)]
#[clap(name = "matroska remux")]
/// Simple Audio Video Encoding tool
struct Opts {
    /// Input file
    #[clap(short, value_parser)]
    input: PathBuf,
    /// Output file
    #[clap(short, value_parser)]
    output: PathBuf,
}

fn main() {
    pretty_env_logger::init();

    let opt = Opts::parse();

    let file = std::fs::File::open(opt.input).unwrap();

    let mut demuxer = DemuxerCtx::new(MkvDemuxer::new(), AccReader::new(file));

    debug!("read headers: {:?}", demuxer.read_headers().unwrap());
    debug!("global info: {:#?}", demuxer.info);

    let mut output = File::create(opt.output).unwrap();

    let mut muxer = muxer::Context::new(
        MkvMuxer::matroska(),
        Writer::from_seekable(Cursor::new(Vec::new())),
    );
    muxer.configure().unwrap();
    muxer.set_global_info(demuxer.info.clone()).unwrap();
    muxer.write_header().unwrap();

    loop {
        match demuxer.read_event() {
            Ok(event) => {
                debug!("event: {:?}", event);
                match event {
                    Event::MoreDataNeeded(sz) => panic!("we needed more data: {} bytes", sz),
                    Event::NewStream(s) => panic!("new stream :{:?}", s),
                    Event::NewPacket(packet) => {
                        debug!("writing packet {:?}", packet);
                        muxer.write_packet(Arc::new(packet)).unwrap();
                    }
                    Event::Continue => {
                        continue;
                    }
                    Event::Eof => {
                        muxer.write_trailer().unwrap();
                        break;
                    }
                    _ => break,
                }
            }
            Err(e) => {
                error!("error: {:?}", e);
                break;
            }
        }
    }

    output
        .write_all(&muxer.writer().seekable_object().unwrap().into_inner())
        .unwrap();
}
