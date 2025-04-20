use std::{thread::sleep, time::Duration};

use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long = "interval", default_value_t = 0)]
    interval: u64,

    #[arg(long = "url")]
    url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    println!("Starting...");

    let interval: Duration = Duration::from_millis(args.interval);
    let url: String = args.url;

    loop {
        println!("Fetching URL: {}", &url);

        let res = reqwest::Client::new().get(&url).send().await?;

        println!("Response: {:?}, {}", res.version(), res.status());

        if !interval.is_zero() {
            sleep(interval);
        } else {
            return Ok(());
        }
    }
}
