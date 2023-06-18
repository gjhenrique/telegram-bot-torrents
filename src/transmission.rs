use hyper::header::AUTHORIZATION;
use hyper::{client, Body, Request, Response};
use serde_json::{json, Value};
use std::env;

use crate::jackett::TorrentLocation;

fn transmission_path(env: String) -> Result<String, String> {
    match env::var(env) {
        Ok(path) => Ok(path),
        Err(_) => {
            Err("TRANSMISSION_TV_PATH or TRANSMISSION_MOVIE_PATH env var is not set".to_string())
        }
    }
}

fn transmission_url() -> String {
    match env::var("TRANSMISSION_URL") {
        Ok(url) => url,
        Err(_) => "http://localhost:9091".to_string(),
    }
}

pub enum Media {
    TV,
    Movie,
}

fn transmission_credentials() -> Option<String> {
    match env::var("TRANSMISSION_CREDENTIALS") {
        Ok(creds) => Some(creds),
        Err(_) => None,
    }
}

async fn request_transmission(
    client: &client::Client<hyper_rustls::HttpsConnector<client::HttpConnector>>,
    location: TorrentLocation,
    path: String,
    token: Option<String>,
) -> hyper::Result<Response<Body>> {
    let creds = transmission_credentials();

    let mut builder = Request::builder()
        .uri(format!("{}/transmission/rpc", transmission_url()))
        .method("POST");

    let headers = builder.headers_mut().unwrap();
    if let Some(creds) = creds {
        let basic = base64::encode(creds);
        let header = format!("Basic {}", basic).parse().unwrap();
        headers.insert(AUTHORIZATION, header);
    }

    if let Some(token) = token {
        headers.insert("X-Transmission-Session-Id", token.parse().unwrap());
    }

    let body: Value;
    if location.is_magnet {
        body = json!({
            "method": "torrent-add",
            "arguments": {
                "download-dir": path,
                "filename": location.content,

            }
        });
    } else {
        body = json!({
            "method": "torrent-add",
            "arguments": {
                "download-dir": path,
                "metainfo": location.content,

            }
        });
    }

    let body = Body::from(body.to_string());
    let request = builder.body(body).unwrap();

    client.request(request).await
}

async fn request_add_torrent(location: TorrentLocation, path: String) -> Result<(), String> {
    let https = hyper_rustls::HttpsConnector::with_native_roots();
    let client: client::Client<_> = client::Client::builder().build(https);

    let transmission_response = request_transmission(&client, location.clone(), path.clone(), None)
        .await;

    if transmission_response.is_err() {
        return Err("Transmission replied with error".to_string());
    }

    let response = transmission_response.unwrap();
    if response.status() == 409 {
        let headers = response.headers();
        let header_value = headers.get("X-Transmission-Session-Id");
        if header_value.is_none() {
            return Err("First request to transmission didn't bring the token {}".to_string());
        }

        let session_value = header_value.unwrap().to_str().unwrap().to_string();
        request_transmission(
            &client,
            location.clone(),
            path.clone(),
            Some(session_value),
        )
        .await
        .unwrap();
        Ok(())
    } else {
        return Err(format!("Error on transmission {}", response.status()));
    }
}

pub async fn add_torrent(location: TorrentLocation, media: Media) -> Result<(), String> {
    let path = match media {
        Media::TV => transmission_path("TRANSMISSION_TV_PATH".to_string())?,
        Media::Movie => transmission_path("TRANSMISSION_MOVIE_PATH".to_string())?,
    };

    request_add_torrent(location, path.clone()).await?;
    Ok(())
}
