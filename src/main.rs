use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(short = 't', long = "toto")]
    toto: String,
}

fn main() {
    let args = Cli::parse();

    println!("toto: {}", args.toto);
}
