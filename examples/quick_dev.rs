use anyhow::Result;
use rodio::Source;
use std::{fs::File, io::BufReader};

use rusty_ytdl::search::YouTube;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let buf = BufReader::new(File::open("data/2I1th_ZXuyk.mp3")?);

    let source = rodio::Decoder::new(buf)?;

    println!("Duration: {:?}", source.total_duration());

    Ok(())
}

// fn pipe() -> (impl Read, impl Write) {
//     //let (tx, rx) = mpsc::channel(1);

//     todo!()
// }
