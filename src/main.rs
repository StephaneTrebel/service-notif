use std::{collections::HashSet, thread::sleep, time::Duration};

use clap::Parser;
use serde::Deserialize;

#[derive(Parser)]
struct Cli {
    #[arg(long = "interval", default_value_t = 0)]
    interval: u64,

    #[arg(long = "cookie-url")]
    cookie_url: String,

    #[arg(long = "search-url")]
    search_url: String,
}

#[derive(Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct TotalItemPrice {
    amount: String,
    currency_code: String,
}

#[derive(Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Thumbnail {
    url: String,
}

#[derive(Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Photo {
    thumbnails: Vec<Thumbnail>,
}

#[derive(Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Item {
    id: usize,
    url: String,
    size_title: String,
    status: String,
    total_item_price: TotalItemPrice,
    photo: Photo
}

#[derive(Deserialize, Debug)]
struct Response {
    items: HashSet<Item>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    println!("Starting...");

    let mut id_set: HashSet<usize> = HashSet::new();

    let interval: Duration = Duration::from_millis(args.interval);
    let cookie_url: String = args.cookie_url;
    let search_url: String = args.search_url;

    let client = reqwest::Client::builder().cookie_store(true).build()?;

    let status = client
        .get(&cookie_url)
        .send()
        .await?
        .status();
    println!("Status: {status}");

    loop {
        println!();

        println!("Fetching URL: {}", &search_url);

        let response_raw = client.get(&search_url).send().await?;
        println!("Response Status: {}", response_raw.status());

        let response = response_raw.json::<Response>().await?;
        // println!("Response object: {:?}", &response);

        let mut new_items = false;

        println!("Length: {}", &response.items.len());

        for item in response.items.iter() {
            if !id_set.contains(&item.id) {
                new_items = true;
                println!("New item {:?}\n", item);
                id_set.insert(item.id);
            }
        }

        if !new_items {
            println!("No new items :(");
        }

        if !interval.is_zero() {
            sleep(interval);
        } else {
            return Ok(());
        }
    }
}
