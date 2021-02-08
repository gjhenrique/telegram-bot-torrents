use std::env;
use std::process::exit;
// use std::fs;
// use std::process::Command;

// use size_format::SizeFormatterSI;
// use url::form_urlencoded;

use futures::StreamExt;

// use hyper::{body::to_bytes, client, Body, Uri};
// use std::str::FromStr;

mod telegram;
mod flexget;

use telegram::handle_message;

use telegram_bot::Api;
use telegram_bot::types::{UpdateKind, MessageKind};

// use telegram_bot::*;
// use telegram_bot::{Api,UpdateKind, ChatId, Message};
// use telegram_bot::types::MessageKind;
// use telegram_bot::types::Message;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let command = &args[1];
        if command == "--version" {
            println!("{}", VERSION);
            exit(0);
        }
    }

    let telegram_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");

    let api = Api::new(telegram_token);
    let mut stream = api.stream();

    while let Some(update) = stream.next().await {
        if let Ok(update) = update {
            match update.kind {
                UpdateKind::Message(message) => match message.kind {
                    MessageKind::Text { ref data, .. } => {
                        let text = data.split_whitespace().map(|s| s.to_string()).collect();

                        handle_message(api.clone(), message.clone(), text).await;
                    }
                    _ => (),
                },
                _ => (),
            }
        };
    }

    Ok(())
}
