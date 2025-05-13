use std::{collections::HashSet, env::var, thread::sleep, time::Duration};

use base64::{engine::general_purpose, Engine};
use clap::Parser;
use maud::html;
use reqwest::{header::HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
struct Cli {
    #[arg(long = "interval", default_value_t = 0)]
    interval: u64,

    #[arg(long = "cookie-url")]
    cookie_url: String,

    #[arg(long = "search-url")]
    search_url: String,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct TotalItemPrice {
    amount: String,
    currency_code: String,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Thumbnail {
    url: String,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Photo {
    thumbnails: Vec<Thumbnail>,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Item {
    id: usize,
    title: String,
    url: String,
    size_title: String,
    status: String,
    total_item_price: TotalItemPrice,
    photo: Photo,
}

#[derive(Serialize, Deserialize, Debug)]
struct Items(HashSet<Item>);

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    items: Items,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    println!("Starting...");

    let mut id_set: HashSet<usize> = HashSet::new();

    let interval: Duration = Duration::from_millis(args.interval);
    let cookie_url: String = args.cookie_url;
    let search_url: String = args.search_url;

    let query_client = reqwest::Client::builder().cookie_store(true).build()?;

    let mut mail_headers = HeaderMap::new();
    mail_headers.insert("Content-Type", "application/json".parse().unwrap());
    mail_headers.insert(
        "Authorization",
        format!(
            "Basic {}",
            general_purpose::STANDARD.encode(
                (format!("{}:{}", var("MAILJET_API_KEY")?, var("MAILJET_API_SECRET")?))
                    .into_bytes()
            )
        )
        .parse()
        .unwrap(),
    );
    let mail_client = reqwest::Client::builder()
        .default_headers(mail_headers)
        .build()?;

    let status = query_client.get(&cookie_url).send().await?.status();
    println!("Status: {status}");

    let mut new_items = false;
    loop {
        println!();

        println!("Fetching URL: {}", &search_url);

        let response_raw = query_client.get(&search_url).send().await?;
        println!("Response Status: {}", response_raw.status());
        let response = response_raw.json::<Response>().await?;

        println!("Length: {}", &response.items.0.len());

        for item in response.items.0.iter() {
            if !id_set.contains(&item.id) {
                new_items = true;
                // println!("New item {:?}\n", item);
                id_set.insert(item.id);
            }
        }

        if !new_items {
            println!("No new items :(");
        } else {
            match send_mail(&mail_client, &response.items).await {
                Ok(_) => println!("Mail correctly sent"),
                Err(e) => println!("Error while sending mail: {}", e),
            }
            new_items = false;
        }

        if !interval.is_zero() {
            sleep(interval);
        } else {
            return Ok(());
        }
    }
}

const MAILJET_SEND_MAIL_URL: &str = "https://api.mailjet.com/v3.1/send";

#[derive(Serialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
struct EMail<'a> {
    email: &'a str,
    name: &'a str,
}

#[derive(Serialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
struct Message<'a> {
    from: EMail<'a>,
    to: Vec<EMail<'a>>,
    subject: &'a str,
    text_part: &'a str,
    #[serde(rename = "HTMLPart")]
    htmlpart: &'a str,
}

#[derive(Serialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
struct Mail<'a> {
    messages: Vec<Message<'a>>,
}

async fn send_mail(mail_client: &reqwest::Client, items: &Items) -> Result<(), reqwest::Error> {
    println!("items: {:?}", serde_json::to_string(items));

    let html_part = create_html_part(items);

    let mail = Mail {
        messages: vec![Message {
            from: EMail {
                email: "service-notif@permacodeur.fr",
                name: "Service Notif",
            },
            to: vec![EMail {
                email: "stephane.trebel@gmail.com",
                name: "You",
            }],
            subject: "New items published !",
            text_part: "New items have been published, go check them !",
            htmlpart: &html_part,
        }],
    };

    println!("JSON: {:?}", serde_json::to_string(&mail));

    let mail_response = mail_client
        .post(MAILJET_SEND_MAIL_URL)
        .json::<Mail>(&mail)
        .send()
        .await;

    match mail_response {
        Ok(response) if response.status() == StatusCode::OK => println!("All good, baby !"),
        Ok(response) if response.status() >= StatusCode::BAD_REQUEST => {
            println!("Error: {:?}", response);
            println!("Response body: {:?}", response.text().await?);
        }
        Err(ref e) => println!(
            "There was catastrophic error while sending the mail: {:?}",
            e
        ),
        response => println!("WTF: {:?}", response),
    };

    Ok(())
}

fn create_html_part(items: &Items) -> String {
    (html! {
        table {
            thead {
                tr {
                    td { "URL" }
                    td { "Taille" }
                    td { "État" }
                    td { "Prix" }
                    td { "Miniature" }
                }
            }
            tbody {
                @for item in &items.0 {
                    tr {
                        td { a href=(item.url) target="_blank" { (item.title) } }
                        td { (item.size_title) }
                        td { (item.status) }
                        td {
                            (item.total_item_price.amount)
                            " "
                            (item.total_item_price.currency_code)
                        }
                        td {
                            img src=(item.photo.thumbnails[0].url) {}
                        }
                    }
                }
            }
        }
    })
    .into_string()
}

#[cfg(test)]
mod tests_create_html_part {
    use std::fs;

    use super::*;

    #[test]
    fn simple() -> Result<(), Box<dyn std::error::Error>> {
        let json = fs::read_to_string("test.json").expect("Cannot load file");
        let items: Items = serde_json::from_str(&json)?;

        let html_part = create_html_part(&items);

        fs::write("test.html", &html_part)?;

        assert_eq!(html_part, "toto");
        Ok(())
    }
}
