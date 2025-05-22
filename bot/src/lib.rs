use std::{collections::HashSet, env::var, thread::sleep, time::Duration};

use base64::{engine::general_purpose, Engine};
use clap::Parser;
use reqwest::{header::HeaderMap, Response, StatusCode};
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

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct Item {
    id: usize,
}

// #[derive(Serialize, Deserialize, Debug)]
// struct Items(HashSet<Item>);

#[derive(Serialize, Deserialize, Debug)]
struct LibResponse {
    items: HashSet<Item>,
}

pub trait GetId {
    fn get_id(&self) -> usize;
}

impl GetId for Item {
    fn get_id(&self) -> usize {
        self.id
    }
}

pub trait CreateHtmlPart {
    fn create_html_part<Items, Inner>(&self, items: &Items) -> String
    where
        Items: IntoIterator<Item = Inner> + Clone,
        Inner: Clone;
}

pub trait ParseResponse {
    async fn parse_response(
        &self,
        response_raw: &Response,
    ) -> Result<LibResponse, Box<dyn std::error::Error>>;
}

pub async fn start<APP>(app: &APP) -> Result<(), Box<dyn std::error::Error>>
where
    APP: CreateHtmlPart + ParseResponse,
{
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
        // TODO let response = response_raw.json::<MyResponse>().await?;
        let response = app.parse_response(&response_raw).await?;

        println!("Length: {}", &response.items.len());

        for item in response.items.iter() {
            if !id_set.contains(&item.id) {
                new_items = true;
                id_set.insert(item.id);
            }
        }

        if !new_items {
            println!("No new items :(");
        } else {
            let html_part = app.create_html_part(&response.items);
            match send_mail(&mail_client, html_part).await {
                Ok(_) => println!("Mail correctly sent"),
                Err(e) => println!("Error while sending mail: {e}"),
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

async fn send_mail(mail_client: &reqwest::Client, html_part: String) -> Result<(), reqwest::Error> {
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
            htmlpart: html_part.as_str(),
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
            println!("Error: {response:?}");
            println!("Response body: {:?}", response.text().await?);
        }
        Err(ref e) => println!("There was catastrophic error while sending the mail: {e}",),
        response => println!("WTF: {response:?}"),
    };

    Ok(())
}
