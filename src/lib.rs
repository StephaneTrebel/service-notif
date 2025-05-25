use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use base64::{engine::general_purpose, Engine};
use maud::html;
use reqwest::{header::HeaderMap, Client, StatusCode};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct ItemCache(Arc<Mutex<HashSet<usize>>>);

impl ItemCache {
    pub fn new() -> Self {
        Self(Arc::default())
    }

    /// Returns true if the given value was not already in the cache
    pub fn insert(&self, key: usize) -> bool {
        self.0.try_lock().unwrap().insert(key)
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TotalItemPrice {
    amount: String,
    currency_code: String,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Thumbnail {
    url: String,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Photo {
    thumbnails: Vec<Thumbnail>,
}

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Item {
    id: usize,
    title: String,
    url: String,
    size_title: String,
    status: String,
    total_item_price: TotalItemPrice,
    photo: Photo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Items(HashSet<Item>);

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    items: Items,
}

pub struct Setup {
    pub query_client: Client,
    pub mail_client: Client,
}

pub async fn setup(
    cookie_url: String,
    mailjet_api_key: String,
    mailjet_api_secret: String,
) -> Result<Setup, Box<dyn std::error::Error>> {
    let mut query_headers = HeaderMap::new();
    query_headers.insert("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:138.0) Gecko/20100101 Firefox/138.0".parse().unwrap());
    let query_client = reqwest::Client::builder()
        .default_headers(query_headers)
        .cookie_store(true).build()?;

    let mut mail_headers = HeaderMap::new();
    mail_headers.insert("Content-Type", "application/json".parse().unwrap());
    mail_headers.insert(
        "Authorization",
        format!(
            "Basic {}",
            general_purpose::STANDARD
                .encode((format!("{}:{}", mailjet_api_key, mailjet_api_secret)).into_bytes())
        )
        .parse()
        .unwrap(),
    );
    let mail_client = reqwest::Client::builder()
        .default_headers(mail_headers)
        .build()?;

    println!("Cookie query URL: {}", &cookie_url);
    let status = query_client.get(&cookie_url).send().await?.status();
    println!("Cookie query status: {status}");
    Ok(Setup {
        query_client,
        mail_client,
    })
}

pub async fn check_and_send_mail(
    query_client: &Client,
    mail_client: &Client,
    search_url: &str,
    item_cache: &ItemCache,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Search query URL: {}", &search_url);
    let response_raw = query_client.get(search_url).send().await?;
    println!("Search query status: {}", response_raw.status());
    let response = response_raw.json::<Response>().await?;

    println!("Length: {}", &response.items.0.len());

    let mut new_items = vec![];
    for item in response.items.0.iter() {
        if item_cache.insert(item.id) {
            new_items.push(item);
        }
    }

    if new_items.is_empty() {
        println!("No new items :(");
    } else {
        match send_mail(mail_client, &new_items).await {
            Ok(_) => println!("Mail correctly sent"),
            Err(e) => println!("Error while sending mail: {}", e),
        }
    }
    Ok(())
}

const MAILJET_SEND_MAIL_URL: &str = "https://api.mailjet.com/v3.1/send";

#[derive(Serialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
pub struct EMail<'a> {
    email: &'a str,
    name: &'a str,
}

#[derive(Serialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
pub struct Message<'a> {
    from: EMail<'a>,
    to: Vec<EMail<'a>>,
    subject: &'a str,
    text_part: &'a str,
    #[serde(rename = "HTMLPart")]
    htmlpart: &'a str,
}

#[derive(Serialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "PascalCase")]
pub struct Mail<'a> {
    messages: Vec<Message<'a>>,
}

pub async fn send_mail(
    mail_client: &reqwest::Client,
    items: &Vec<&Item>,
) -> Result<(), reqwest::Error> {
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

pub fn create_html_part(items: &Vec<&Item>) -> String {
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
                @for item in items {
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
        let raw_items: Vec<Item> = serde_json::from_str(&json)?;
        let items: Vec<&Item> = raw_items.iter().collect();

        let html_part = create_html_part(&items);

        fs::write("test.html", &html_part)?;

        assert_eq!(html_part, "toto");
        Ok(())
    }
}
