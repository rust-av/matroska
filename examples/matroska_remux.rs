#[macro_use]
extern crate log;

use av_format::{buffer::AccReader, demuxer::Event, muxer};

use av_format::demuxer::Context as DemuxerCtx;
use matroska::{demuxer::MkvDemuxer, muxer::MkvMuxer};
use std::{fs::File, sync::Arc};

// Command line interface
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "matroska remux")]
/// Simple Audio Video Encoding tool
struct Opt {
    /// Input file
    #[structopt(short = "i", parse(from_os_str))]
    input: PathBuf,
    /// Output file
    #[structopt(short = "o", parse(from_os_str))]
    output: PathBuf,
}

fn main() {
    pretty_env_logger::init();

    let opt = Opt::from_args();

    let file = std::fs::File::open(opt.input).unwrap();

    let acc = AccReader::new(file);

    let mut demuxer = DemuxerCtx::new(Box::new(MkvDemuxer::new()), Box::new(acc));

    debug!("read headers: {:?}", demuxer.read_headers().unwrap());
    debug!("global info: {:#?}", demuxer.info);

    let mux = Box::new(MkvMuxer::matroska());

    let output = File::create(opt.output).unwrap();

    let mut muxer = muxer::Context::new(mux, Box::new(output));
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
                    _ => break
                }
            }
            Err(e) => {
                error!("error: {:?}", e);
                break;
            }
        }
    }
}
