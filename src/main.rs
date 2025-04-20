use std::{
    thread::sleep,
    time::Duration,
};

use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(short = 'i', long = "interval")]
    interval: u64,
}

fn main() {
    let args = Cli::parse();
    println!("Starting...");

    let interval: Duration = Duration::from_millis(args.interval);

    loop {
        println!("interval: {}", args.interval);
        sleep(interval);
    }
}
