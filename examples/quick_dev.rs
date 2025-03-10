use anyhow::Result;

use rusty_ytdl::search::YouTube;

#[tokio::main]
async fn main() -> Result<()> {
    let youtube = YouTube::new().unwrap();

    let res = youtube.search("The beach - 0171", None).await;

    println!("{:?}", res);

    Ok(())
}

// fn pipe() -> (impl Read, impl Write) {
//     //let (tx, rx) = mpsc::channel(1);

//     todo!()
// }
