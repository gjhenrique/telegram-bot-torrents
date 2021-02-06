use std::env;
use std::fs;
use std::process::exit;
use std::process::Command;

use url::form_urlencoded;

use rand::Rng;

use futures::StreamExt;

use hyper::{body::to_bytes, client, Body, Uri};
use std::str::FromStr;

use telegram_bot::*;
// use telegram_bot::{Api,UpdateKind, ChatId, Message};
// use telegram_bot::types::MessageKind;
// use telegram_bot::types::Message;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn flexget_command(flexget_path: &str, flexget_command: &str) {
    let command = Command::new("sh")
        .arg("-c")
        .arg(format!("flexget execute {}", flexget_command))
        .current_dir(flexget_path)
        .output();

    match command {
        Ok(c) => {
            println!("Stderr: {}", String::from_utf8_lossy(&c.stderr));
            println!("Stdout: {}", String::from_utf8_lossy(&c.stdout));
        }
        Err(err) => {
            println!("{}", err);
            return;
        }
    }
}

async fn dispatch_sync(api: Api, message: Message, flexget_path: &str) -> Result<(), Error> {
    flexget_command(flexget_path, "--no-cache --discover-now");

    api.send(message.chat.text("Finished syncing")).await?;

    Ok(())
}

async fn dispatch_movie(text: Vec<&str>, flexget_path: &str) -> Result<(), Error> {
    if text.len() <= 1 {
        return Ok(());
    }

    let magnet_url = text[1];
    let argument = format!(
        "--task download-movie-manual --cli-config \"magnet={}\"",
        magnet_url
    );
    println!("{}", argument);
    flexget_command(flexget_path, &argument);

    Ok(())
}

async fn dispatch_tv(text: Vec<&str>, flexget_path: &str) -> Result<(), Error> {
    if text.len() <= 1 {
        return Ok(());
    }

    let magnet_url = text[1];
    let argument = format!(
        "--task download-tv-manual --cli-config 'magnet={}'",
        magnet_url
    );
    flexget_command(flexget_path, &argument);

    Ok(())
}

async fn dispatch_chat_id(api: Api, message: Message) -> Result<(), Error> {
    let chat_id = message.chat.id();
    let text = format!("Chat ID: {}", chat_id.to_string());

    api.send(message.chat.text(text)).await?;

    Ok(())
}

fn omdb_token() -> String {
    match env::var("OMDB_TOKEN") {
        Ok(token) => token,
        Err(_) => String::new(),
    }
}

fn imdb_title(imdb_url: String) -> String {
    // if url does has an error, answer with the error of the body

    // imdb_url

    let url = imdb_url.parse::<Uri>().unwrap();
    return url
        .path()
        .split("/")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>()
        .last()
        .unwrap()
        .clone();
}

#[derive(serde::Deserialize)]
struct OmdbData {
    #[serde(rename(deserialize = "Response"))]
    response: Option<String>,
    #[serde(rename(deserialize = "Title"))]
    title: Option<String>,
    #[serde(rename(deserialize = "Year"))]
    year: Option<String>,
    #[serde(rename(deserialize = "Error"))]
    error: Option<String>,
}

async fn get_imdb_info(
    imdb_url: String,
    client: client::Client<hyper_rustls::HttpsConnector<client::HttpConnector>>,
) -> Result<String, Error> {
    let title = imdb_title(imdb_url);
    let token = omdb_token();

    // if omdb api token is not set, say that the token needs to be configured

    let omdb_url = format!("http://www.omdbapi.com/?apikey={}&i={}", token, title);
    let res = client.get(Uri::from_str(&omdb_url).unwrap()).await.unwrap();

    let body: Body = res.into_body();
    let body = to_bytes(body).await.unwrap();

    let str = String::from_utf8_lossy(&body);
    let v: OmdbData = serde_json::from_str(&str).unwrap();
    match v.response.unwrap().as_ref() {
        "True" => {
            return Ok(format!("{} ({})", v.title.unwrap(), v.year.unwrap()));
        }
        "False" => {
            return Ok(v.error.unwrap());
        }
        _ => {
            return Ok(String::from(str));
        }
    }
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

#[derive(serde::Deserialize)]
struct Indexer {
    #[serde(rename(deserialize = "Name"))]
    name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename(deserialize = "Results"))]
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
    category: Vec<i64>,
    #[serde(rename(deserialize = "Details"))]
    detail_url: String,
}

#[derive(serde::Deserialize)]
struct JackettResponse {
    #[serde(rename(deserialize = "Indexers"))]
    indexers: Vec<Indexer>,
    #[serde(rename(deserialize = "Results"))]
    results: Vec<Torrent>,
}

struct TelegramJackettResponse {
    identifier: String,
    torrents: Vec<Torrent>,
}

fn random_chars() -> String {
    return rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(5)
        .map(char::from)
        .collect();
}

async fn request_jackett(
    query_string: String,
    client: client::Client<hyper_rustls::HttpsConnector<client::HttpConnector>>,
) -> Result<TelegramJackettResponse, String> {
    let encoded_path: String = form_urlencoded::Serializer::new(String::new())
        .append_pair("apikey", jackett_token().unwrap().as_str())
        .append_pair("Query", query_string.as_str())
        .finish();

    let url = [
        jackett_host(),
        String::from("/api/v2.0/indexers/all/results?"),
        encoded_path,
    ]
    .join("");

    let res = client.get(Uri::from_str(&url).unwrap()).await.unwrap();
    let body: Body = res.into_body();
    let body = to_bytes(body).await.unwrap();
    let str = String::from_utf8_lossy(&body);

    let mut v: JackettResponse = serde_json::from_str(&str).unwrap();
    if v.indexers.len() == 0 && v.results.len() == 0 {
        return Err("Add some indexers".to_string());
    }

    v.results.sort_by_key(|d1| -d1.seeders);
    let torrents = v.results.into_iter().take(20).collect();

    let response = TelegramJackettResponse {
        identifier: random_chars(),
        torrents,
    };

    return Ok(response);
}

#[tokio::main]
async fn main() -> Result<(), telegram_bot::Error> {
    let url = "https://www.imdb.com/title/tt0347048/?ref_=hm_tpks_tt_1_pd_tp1_cp";

    let https = hyper_rustls::HttpsConnector::new();
    let client: client::Client<_, hyper::Body> = client::Client::builder().build(https);

    request_jackett("Lord of the rings".to_string(), client)
        .await
        .unwrap();

    exit(0);

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let command = &args[1];
        if command == "--version" {
            println!("{}", VERSION);
            exit(0);
        }
    }

    let allowed_groups: Vec<ChatId> = match env::var("TELEGRAM_ALLOWED_GROUPS") {
        Ok(val) => val
            .split(",")
            .map(|x| ChatId::new(x.parse::<i64>().unwrap()))
            .collect::<Vec<ChatId>>(),
        Err(_) => Vec::new(),
    };

    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let flexget_path = env::var("FLEXGET_PATH").expect("FLEXGET_PATH not set");

    let api = Api::new(token);
    let mut stream = api.stream();

    while let Some(update) = stream.next().await {
        // If the received update contains a new message...
        let update = update?;

        match update.kind {
            UpdateKind::Message(message) => match message.kind {
                MessageKind::Text { ref data, .. } => {
                    let text = data.split_whitespace().collect::<Vec<&str>>();
                    let prefix = text[0];
                    let chat_id = message.chat.id();

                    if prefix == "/chat-id" {
                        dispatch_chat_id(api.clone(), message.clone()).await?;
                    }

                    if allowed_groups.is_empty() || allowed_groups.contains(&chat_id) {
                        match prefix {
                            "/tv" => dispatch_tv(text, &flexget_path).await?,
                            "/movie" => dispatch_movie(text, &flexget_path).await?,
                            "/sync" => {
                                dispatch_sync(api.clone(), message.clone(), &flexget_path).await?
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            },
            _ => (),
        }
    }

    Ok(())
}
