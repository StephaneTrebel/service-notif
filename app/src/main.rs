use maud::html;
use std::collections::HashSet;

use reqwest::Response;
use serde::{Deserialize, Serialize};

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
struct MyResponse {
    items: Items,
}

struct MyApp;

impl bot::CreateHtmlPart for MyApp {
    // fn create_html_part<T>(items: &std::collections::HashSet<T>) -> String {
    // todo!()
    // }
    fn create_html_part(&self, items: &Items) -> String {
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
}

impl bot::ParseResponse for MyApp {
    async fn parse_response(
        &self,
        response_raw: &Response,
    ) -> Result<MyResponse, Box<dyn std::error::Error>> {
        let response = response_raw.json::<MyResponse>().await?;
        response
    }
}

// #[cfg(test)]
// mod tests_create_html_part {
// use std::fs;

// use super::*;

// #[test]
// fn simple() -> Result<(), Box<dyn std::error::Error>> {
// let json = fs::read_to_string("test.json").expect("Cannot load file");
// let items: Items = serde_json::from_str(&json)?;

// let html_part = create_html_part(&items);

// fs::write("test.html", &html_part)?;

// assert_eq!(html_part, "toto");
// Ok(())
// }
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let my_app = MyApp {};
    bot::start(&my_app).await
}
