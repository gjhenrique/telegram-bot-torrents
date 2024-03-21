use std::env;
use std::process::exit;

use futures::lock::Mutex;
use futures::StreamExt;

mod imdb;
mod jackett;
mod telegram;
mod transmission;

use telegram::handle_message;

use std::error::Error;
use std::sync::Arc;
use telegram_bot::types::{MessageKind, UpdateKind};
use telegram_bot::{AllowedUpdate, Api, UpdatesStream};

use crate::jackett::TelegramJackettResponse;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

    let tracing = env::var("ENABLE_TRACING").is_ok();

    if tracing {
        tracing::subscriber::set_global_default(
            tracing_subscriber::FmtSubscriber::builder()
                .with_env_filter("telegram_bot=trace")
                .finish(),
        )
        .unwrap();
    }

    let responses: Arc<Mutex<Vec<TelegramJackettResponse>>> = Arc::new(Mutex::new(Vec::new()));

    let telegram_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");

    let api = Api::new(telegram_token);
    let mut stream = UpdatesStream::new(&api);
    stream.allowed_updates(&[AllowedUpdate::Message]);

    while let Some(update) = stream.next().await {
        let Ok(update) = update else {
            continue;
        };

        let UpdateKind::Message(message) = update.kind else {
            continue;
        };

        let MessageKind::Text { ref data, .. } = message.kind else {
            continue;
        };

        let text = data.split_whitespace().map(|s| s.to_string()).collect();
        let cloned_api = api.clone();
        let mut shared_responses = Arc::clone(&responses);
        let data_cloned = data.clone();

        tokio::spawn(async move {
            let handle = handle_message(&cloned_api, &message, text, &mut shared_responses);
            if (handle.await).is_err() {
                let error_msg = format!(
                    "Errors should be handled in handle_message {:?}",
                    data_cloned.clone()
                );
                println!("{}", error_msg);
            };
        });
    }

    Ok(())
}
