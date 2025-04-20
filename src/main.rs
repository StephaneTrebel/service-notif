use std::{thread::sleep, time::Duration};

use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long = "interval")]
    interval: u64,

    #[arg(long = "url")]
    url: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    println!("Starting...");

    let interval: Duration = Duration::from_millis(args.interval);
    let url: String = args.url;

    loop {
        println!("Fetching URL: {}", &url);

        let res = reqwest::blocking::get(&url)?;
        println!("Response: {:?}, {}", res.version(), res.status());

        sleep(interval);
    }
}
