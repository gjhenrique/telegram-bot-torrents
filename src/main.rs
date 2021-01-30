use std::env;
use std::process::Command;

use futures::StreamExt;

use telegram_bot::*;

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

#[tokio::main]
async fn main() -> Result<(), Error> {
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
