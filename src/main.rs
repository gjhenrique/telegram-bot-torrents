use std::env;
use std::fs;
use std::process::exit;
use std::process::Command;

use size_format::SizeFormatterSI;
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

#[tokio::main]
async fn main() -> Result<(), telegram_bot::Error> {
    // let url = "https://www.imdb.com/title/tt0347048/?ref_=hm_tpks_tt_1_pd_tp1_cp";

    let mut torrents: Vec<TelegramJackettResponse> = Vec::new();
    let https = hyper_rustls::HttpsConnector::new();
    let client: client::Client<_, hyper::Body> = client::Client::builder().build(https);

    let response = request_jackett("Lord of the rings".to_string(), client)
        .await
        .unwrap();
    torrents.push(response.clone());

    // exit(0);

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

                    if prefix == "/help" {
                        dispatch_search(api.clone(), message.clone(), response.clone()).await?;
                    }

                    if allowed_groups.is_empty() || allowed_groups.contains(&chat_id) {
                        let num = prefix.parse::<u16>();
                        if num.is_ok() && message.clone().reply_to_message.is_some() {
                            dispatch_from_reply(
                                api.clone(),
                                message.clone(),
                                num.unwrap(),
                                torrents.clone(),
                            )
                            .await?
                        }

                        match prefix {
                            "/torrent-tv" => dispatch_tv(text, &flexget_path).await?,
                            "/torrent-movie" => dispatch_movie(text, &flexget_path).await?,
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
