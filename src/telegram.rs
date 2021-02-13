use std::env;

use telegram_bot::prelude::*;
use telegram_bot::{Api, ChatId, Message, ParseMode};

use crate::flexget::{execute_magnet_url, sync_flexget, Media};
use crate::jackett::{request_jackett, format_telegram_response, dispatch_from_reply, TelegramJackettResponse};
use crate::imdb::get_imdb_info;

fn allowed_groups() -> Vec<ChatId> {
    return match env::var("TELEGRAM_ALLOWED_GROUPS") {
        Ok(val) => val
            .split(",")
            .map(|x| ChatId::new(x.parse::<i64>().unwrap()))
            .collect::<Vec<ChatId>>(),
        Err(_) => Vec::new(),
    };
}

async fn dispatch_chat_id(message: Message) -> Result<String, String> {
    let chat_id = message.chat.id();
    let reply = format!("Chat ID: {}", chat_id.to_string());

    Ok(reply)
}


async fn dispatch_sync() -> Result<String, String> {
    sync_flexget()?;

    Ok("Finished syncing".to_string())
}


async fn dispatch_tv(text: Vec<String>) -> Result<String, String> {
    if text.len() <= 1 {
        return Err("Send the magnet-url after command (/torrent-tv <magnet_url>)".to_string());
    }

    execute_magnet_url(text[1].clone(), Media::TV)?;

    Ok("Magnet URL to Series".to_string())
}

async fn dispatch_movie(text: Vec<String>) -> Result<String, String> {
    if text.len() <= 1 {
        return Err("Send the magnet-url after command (/torrent-movie <magnet_url>)".to_string());
    }

    execute_magnet_url(text[1].clone(), Media::Movie)?;

    Ok("Magnet URL to Movies".to_string())
}

async fn dispatch_from_imdb_url(imdb_url: String) -> Result<TelegramJackettResponse, String> {
    let title = get_imdb_info(imdb_url.clone()).await?;
    let result = request_jackett(title).await?;

    Ok(result)
}

async fn dispatch_search(text: Vec<String>) -> Result<TelegramJackettResponse, String> {
    if text.len() <= 1 {
        return Err("Pass the movie/TV after command (/search Matrix 1999)".to_string());
    }

    let search_text = text[1..].join(" ");
    let result = request_jackett(search_text).await?;

    Ok(result)
}

async fn pick_choices(index:  u16, reply_text: String, torrents: Vec<TelegramJackettResponse>) -> Result<String, String> {
    let (media, magnet_url) = dispatch_from_reply(index, reply_text, torrents).await?;

    execute_magnet_url(magnet_url, media)?;

    Ok("Added torrent".to_string())
}

pub async fn send_message(api: &Api, message: &Message, text: String) -> Result<(), ()> {
    let mut reply = message.text_reply(text);

    let result = api.send(reply.parse_mode(ParseMode::Html)).await;
    match result {
        Ok(_) => {
            println!("Reply: {:?}", reply);
            Ok(())
        }
        Err(err) => {
            println!("Error when sending telegram message: {}", err);
            Ok(())
        }
    }
}

fn add_response(response: Result<TelegramJackettResponse, String>, responses: &mut Vec<TelegramJackettResponse>) -> Result<String, String> {
    match response {
        Ok(response) => {
            let reply_text = format_telegram_response(response.clone());
            responses.push(response);
            Ok(reply_text)
        }
        Err(err) => {
            Err(err)
        }
    }
}

pub async fn handle_message(api: &Api, message: &Message, text: Vec<String>, responses: &mut Vec<TelegramJackettResponse> ) -> Result<(), ()> {
    let chat_id = message.chat.id();
    let mut result: Result<String, String> = Err("I didn't get it!".to_string());

    let prefix = text.first().unwrap();
    let suffix = text.last().unwrap();

    if prefix.as_str() == "/chat-id" {
        result = dispatch_chat_id(message.clone()).await;
    }

    if allowed_groups().is_empty() || allowed_groups().contains(&chat_id) {
        if let Some(reply) = message.reply_to_message.clone() {
            let num = prefix.parse::<u16>();
            if let Ok(num) = num {
                if let Some(reply_text) = reply.text() {
                    result = pick_choices(num, reply_text, responses.clone()).await;
                }
            }
        }

        // TODO: Move to const
        let imdb_url  = "https://www.imdb.com";
        if prefix.starts_with(imdb_url) || suffix.starts_with(imdb_url) || prefix == "/imdb" {
            let mut url = suffix;

            if prefix.starts_with(imdb_url) {
                url = prefix;
            }

            let response = dispatch_from_imdb_url(url.clone()).await;
            result = add_response(response, responses)
        };

        result = match prefix.as_str() {
            "/torrent-tv" => dispatch_tv(text).await,
            "/torrent-movie" => dispatch_movie(text).await,
            "/sync" => dispatch_sync().await,
            // TODO: Add help
            "/search" => {
                let response = dispatch_search(text).await;
                add_response(response, responses)
            }
            _ => result,
        };

    }

    println!("{:?}", result);
    match result {
        Ok(text) => {
            if text != "" {
                send_message(&api, &message, text.clone()).await?;
            }
        }
        Err(text) => {
            send_message(&api, &message, text.clone()).await?;
        }
    };
    return Ok(());
}
