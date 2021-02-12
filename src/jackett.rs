use hyper::{body::to_bytes, client, Body, Uri};
use std::env;
use std::fs;

use size_format::SizeFormatterSI;
use std::str::FromStr;
use url::form_urlencoded;

use crate::flexget::Media;

#[derive(serde::Deserialize)]
struct Indexer {
    #[serde(rename(deserialize = "Name"))]
    name: String,
}

#[derive(serde::Deserialize, Clone)]
struct Torrent {
    #[serde(rename(deserialize = "Seeders"))]
    seeders: i64,
    #[serde(rename(deserialize = "Peers"))]
    peers: i64,
    #[serde(rename(deserialize = "MagnetUri"))]
    magnet_uri: String,
    #[serde(rename(deserialize = "Title"))]
    title: String,
    #[serde(rename(deserialize = "Category"))]
    categories: Vec<i64>,
    #[serde(rename(deserialize = "Details"))]
    detail_url: String,
    #[serde(rename(deserialize = "Size"))]
    size: u64,
}

#[derive(serde::Deserialize)]
struct JackettResponse {
    #[serde(rename(deserialize = "Indexers"))]
    indexers: Vec<Indexer>,
    #[serde(rename(deserialize = "Results"))]
    results: Vec<Torrent>,
}

#[derive(Clone)]
pub struct TelegramJackettResponse {
    torrents: Vec<Torrent>,
}

fn jackett_host() -> String {
    match env::var("JACKETT_HOST") {
        Ok(host) => host,
        Err(_) => String::from("http://localhost:9117"),
    }
}

fn jackett_token() -> Result<String, String> {
    match env::var("JACKETT_TOKEN") {
        Ok(token) => Ok(token),
        Err(_) => match env::var("JACKETT_DATA_DIR") {
            Ok(data_dir) => {
                let file_name = data_dir + "/ServerConfig.json";

                let file_content = fs::read_to_string(file_name);

                let v: serde_json::Value =
                    serde_json::from_str(file_content.unwrap().as_str()).unwrap();

                if v["APIKey"] == serde_json::Value::Null {
                    Err(String::from("Jackett file does not have key APIKey"))
                } else {
                    Ok(v["APIKey"].as_str().unwrap().to_string())
                }
            }
            Err(_) => Err(String::from(
                "Set JACKETT_TOKEN or JACKETT_DATA_DIR if jackett is in the same host",
            )),
        },
    }
}


pub async fn request_jackett(
    query_string: String,
) -> Result<TelegramJackettResponse, String> {
    let https = hyper_rustls::HttpsConnector::new();
    let client: client::Client<_, hyper::Body> = client::Client::builder().build(https);

    let encoded_path: String = form_urlencoded::Serializer::new(String::new())
        .append_pair("apikey", jackett_token().unwrap().as_str())
        .append_pair("Query", query_string.as_str())
        .finish();

    let url = [
        jackett_host(),
        String::from("/api/v2.0/indexers/all/results?"),
        encoded_path,
    ].join("");

    let res = client.get(Uri::from_str(&url).unwrap()).await.unwrap();
    let body: Body = res.into_body();
    let body = to_bytes(body).await.unwrap();
    let str = String::from_utf8_lossy(&body);

    let mut v: JackettResponse = serde_json::from_str(&str).unwrap();
    if v.indexers.len() == 0 && v.results.len() == 0 {
        return Err("Empty indexers. Please add one in your jackett configuration".to_string());
    }

    v.results.sort_by_key(|d1| -d1.seeders);
    let torrents = v.results.into_iter().take(20).collect();

    let response = TelegramJackettResponse { torrents };

    return Ok(response);
}

pub fn format_telegram_response(
    response: TelegramJackettResponse,
) -> String {
    let info = format_torrent(response);

    format!("<pre>{}</pre>", info)
}

fn format_torrent(response: TelegramJackettResponse) -> String {
    return response
        .torrents
        .iter()
        .enumerate()
        .fold(String::from(""), |text, (i, t)| {
            text + format!(
                "{}. {} - {}B - {}\n",
                i + 1,
                t.title,
                SizeFormatterSI::new(t.size),
                t.seeders
            )
            .as_str()
        });
}


fn is_movie(categories: Vec<i64>) -> bool {
    return categories.iter().any(|c| c >= &2000 && c < &3000);
}

fn is_tv_show(categories: Vec<i64>) -> bool {
    return categories.iter().any(|c| c >= &3000 && c < &4000);
}

pub async fn dispatch_from_reply(
    index: u16,
    reply_text: String,
    torrents: Vec<TelegramJackettResponse>,
) -> Result<(Media, String), String> {
    let real_index = index - 1;

    let jackett = torrents.clone().into_iter().find(|response| {
        format_torrent(response.clone())
            .split_whitespace()
            .collect::<String>()
            == reply_text.split_whitespace().collect::<String>()
    });

    match jackett {
        Some(jackett) => {
            let torrent = jackett.torrents.iter().nth(real_index.into()).unwrap();

            if is_tv_show(torrent.clone().categories) {
                Ok((Media::TV, torrent.clone().magnet_uri))
            } else if is_movie(torrent.clone().categories) {
                Ok((Media::Movie, torrent.clone().magnet_uri))
            } else {
                Err("Category not found".to_string())
            }
        },
        None => {
            Err("Couldn't find torrent".to_string())
        }
    }
}

    // let url = "https://www.imdb.com/title/tt0347048/?ref_=hm_tpks_tt_1_pd_tp1_cp";
    // let mut torrents: Vec<TelegramJackettResponse> = Vec::new();
    // let https = hyper_rustls::HttpsConnector::new();
    // let client: client::Client<_, hyper::Body> = client::Client::builder().build(https);

    // let response = request_jackett("Lord of the rings".to_string(), client)
    //     .await
    //     .unwrap();
    // torrents.push(response.clone());
