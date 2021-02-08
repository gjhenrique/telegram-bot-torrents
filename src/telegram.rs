use std::env;

use telegram_bot::prelude::*;
use telegram_bot::{Api, ChatId, Message};

use crate::flexget::{execute_magnet_url, sync_flexget, Media};

fn allowed_groups() -> Vec<ChatId> {
    return match env::var("TELEGRAM_ALLOWED_GROUPS") {
        Ok(val) => val
            .split(",")
            .map(|x| ChatId::new(x.parse::<i64>().unwrap()))
            .collect::<Vec<ChatId>>(),
        Err(_) => Vec::new(),
    };
}

async fn dispatch_chat_id(message: Message) -> Result<String, &'static str> {
    let chat_id = message.chat.id();
    let string = format!("Chat ID: {}", chat_id.to_string());

    return Ok(string);
}


async fn dispatch_sync(api: Api, message: Message) -> Result<&'static str, &'static str> {
    sync_flexget()?;

    return Ok("Finished syncing");
}


async fn dispatch_tv(api: Api, message: Message, text: Vec<String>) -> Result<&'static str, &'static str> {
    if text.len() <= 1 {
        return Err("Send the magnet-url after command (/torrent-tv <magnet_url>");
    }

    execute_magnet_url(text[1].clone(), Media::TV)?;

    return Ok("Magnet URL to Series");
    // return match api.send(message.chat.text("Magnet URL to TV")).await {
    //     Ok(_) => Ok(()),
    //     Err(e) => {
    //         println!("#{:?}", e);
    //         return Err("Telegram error");
    //     }
    // };
}

async fn dispatch_movie(api: Api, message: Message, text: Vec<String>) -> Result<&'static str, &'static str> {
    if text.len() <= 1 {
        return Err("Send the magnet-url after command (/torrent-movie <magnet_url>");
    }

    execute_magnet_url(text[1].clone(), Media::Movie)?;

    return Ok("Magnet URL to Movies");
    // return match api.send(message.chat.text("Magnet URL to Movies")).await {
    //     Ok(_) => Ok(()),
    //     Err(e) => {
    //         println!("#{:?}", e);
    //         return Err("Telegram error");
    //     }
    // };
}

pub async fn handle_message(api: Api, message: Message, text: Vec<String>) -> Result<(), &'static str> {
    let prefix = &text.clone()[0];
    let chat_id = message.chat.id();

    if prefix == "/chat-id" {
        dispatch_chat_id(message.clone()).await?;
    }

    // if prefix == ", String/help" {
    //     dispatch_search(api.clone(), message.clone(), response.clone()).await?;
    // }

    //     if message.clone().reply_to_message.is_some() {
    //         // let num = prefix.parse::<u16>();
    //         // num.is_ok()
    //         dispatch_from_reply(
    //             api.clone(),
    //             message.clone(),
    //             num.unwrap(),
    //             torrents.clone(),
    //         ).await?
    //     }

    println!("{}", prefix);
    if allowed_groups().is_empty() || allowed_groups().contains(&chat_id) {
        let result = match prefix.as_str() {
            "/torrent-tv" => dispatch_tv(api.clone(), message.clone(), text).await,
            "/torrent-movie" => dispatch_movie(api.clone(), message.clone(), text).await,
            "/sync" => dispatch_sync(api.clone(), message.clone()).await,
            _ => Ok(""),
        };
    }

    return Ok(());
}
