use std::env;
use std::process::exit;

use futures::StreamExt;

mod telegram;
mod flexget;
mod jackett;
mod imdb;

use telegram::handle_message;

use telegram_bot::Api;
use telegram_bot::types::{UpdateKind, MessageKind};
use std::error::Error;

use crate::jackett::TelegramJackettResponse;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let command = &args[1];
        if command == "--version" {
            println!("{}", VERSION);
            exit(0);
        }
    }

    let mut responses: Vec<TelegramJackettResponse> = Vec::new();

    let telegram_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");

    let api = Api::new(telegram_token);
    let mut stream = api.stream();

    while let Some(update) = stream.next().await {
        if let Ok(update) = update {
            match update.kind {
                UpdateKind::Message(message) => match message.kind {
                    MessageKind::Text { ref data, .. } => {
                        let text = data.split_whitespace().map(|s| s.to_string()).collect();

                        if let Err(_) = handle_message(&api, &message, text, &mut responses).await {
                            let error_msg = format!("Errors should be handled in handle_message {:?}", data);
                            println!("{}", error_msg);
                        };
                    }
                    _ => (),
                },
                _ => (),
            }
        };
    }

    Ok(())
}
