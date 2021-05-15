use std::env;

use telegram_bot::prelude::*;
use telegram_bot::{Api, ChatId, Message, ParseMode};

use crate::flexget::sync_flexget;
use crate::imdb::get_imdb_info;
use crate::jackett::{
    dispatch_from_reply, format_telegram_response, request_jackett, TelegramJackettResponse,
};
use crate::transmission::{add_torrent, Media};

const HELP: &str = "
/torrent-tv (Magnet Link)
/torrent-movie (Magnet Link)
/search (Movie or TV Show e.g. The Matrix or Simpsons s01e01)
/imdb (Imdb link). Requires omdb token set https://www.omdbapi.com/

Reply the magnet links with:
Position of the torrent
If jackett doesn't provide a category, it's possible to force with:
tv (position)
movie (position)
";

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

    Ok("🧲 Added torrent".to_string())
}

async fn dispatch_tv(text: Vec<String>) -> Result<String, String> {
    if text.len() <= 1 {
        return Err("Send the magnet-url after command (/torrent-tv magnet_url)".to_string());
    }

    add_torrent(text[1].clone(), Media::TV).await?;

    Ok("🧲 Added torrent".to_string())
}

async fn dispatch_movie(text: Vec<String>) -> Result<String, String> {
    if text.len() <= 1 {
        return Err("Send the magnet-url after command (/torrent-movie magnet_url)".to_string());
    }

    add_torrent(text[1].clone(), Media::Movie).await?;

    Ok("🧲 Added torrent".to_string())
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

async fn pick_choices(
    index: u16,
    reply_text: String,
    torrents: Vec<TelegramJackettResponse>,
    mut media: Option<Media>
) -> Result<String, String> {
    let (torrent_media, magnet_url) = dispatch_from_reply(index, reply_text, torrents).await?;

    if media.is_none() && torrent_media.is_none() {
        return Err("No category for given torrent.\nReply with tv (index) or movie (index) to force it".to_string());
    }

    if media.is_none() {
        media = torrent_media;
    }

    add_torrent(magnet_url, media.unwrap()).await?;

    Ok("🧲 Added torrent".to_string())
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

fn add_response(
    response: Result<TelegramJackettResponse, String>,
    responses: &mut Vec<TelegramJackettResponse>,
) -> Result<String, String> {
    match response {
        Ok(response) => {
            let reply_text = format_telegram_response(response.clone());
            responses.push(response);
            Ok(reply_text)
        }
        Err(err) => Err(err),
    }
}

pub async fn handle_message(
    api: &Api,
    message: &Message,
    text: Vec<String>,
    responses: &mut Vec<TelegramJackettResponse>,
) -> Result<(), ()> {
    let chat_id = message.chat.id();
    let mut result: Result<String, String> = Err("🤷🏻‍I didn't get it!".to_string());

    let prefix = text.first().unwrap();
    let suffix = text.last().unwrap();

    if prefix.as_str() == "/chat-id" {
        result = dispatch_chat_id(message.clone()).await;
    }

    if allowed_groups().is_empty() || allowed_groups().contains(&chat_id) {
        if let Some(reply) = message.reply_to_message.clone() {
            let num: Option<u16>;
            let mut media: Option<Media> = None;

            match prefix.as_str() {
                "tv" => {
                    media = Some(Media::TV);
                    num =  suffix.parse::<u16>().ok();
                }
                "movie" => {
                    media = Some(Media::Movie);
                    num = suffix.parse::<u16>().ok();
                }
                _ => {
                    num = prefix.parse::<u16>().ok();
                }
            }

            if let Some(num) = num {
                if let Some(reply_text) = reply.text() {
                    result = pick_choices(num, reply_text, responses.clone(), media).await;
                }
            } else {
                result = Err("Not a number.\nPossible solutions: (index), movie (index) or tv (index) ".to_string())
            }
        }

        // TODO: Move to const
        let imdb_url = "https://www.imdb.com";
        if prefix.starts_with(imdb_url)
            || suffix.starts_with(imdb_url)
            || (prefix == "/imdb" || suffix.starts_with(imdb_url))
        {
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
            "/help" => Ok(HELP.to_string()),
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
            send_message(&api, &message, format!("❌ {}", text.clone())).await?;
        }
    };
    return Ok(());
}
