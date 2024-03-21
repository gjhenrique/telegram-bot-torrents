use hyper::{body::to_bytes, client, Body, Uri};
use std::env;
use std::str::FromStr;

fn omdb_token() -> Result<String, String> {
    match env::var("OMDB_TOKEN") {
        Ok(token) => Ok(token),
        Err(_) => Err("OMDB_TOKEN env var is not configured".to_string()),
    }
}

fn imdb_title(imdb_url: String) -> Result<String, String> {
    let uri = imdb_url
        .parse::<Uri>()
        .map_err(|err| format!("Undefined IMDB url {}", err))?;

    let fragments = uri
        .path()
        .split('/')
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<String>>();

    let imdb_title = fragments.last();

    match imdb_title {
        Some(title) => Ok(title.to_string()),
        None => Err("Couldn't find the imdb id from the url".into()),
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
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

pub async fn get_imdb_info(imdb_url: String) -> Result<String, String> {
    let https = hyper_rustls::HttpsConnector::with_native_roots();
    let client: client::Client<_> = client::Client::builder().build(https);

    let token = omdb_token()?;
    let title = imdb_title(imdb_url)?;

    let omdb_url = format!("http://www.omdbapi.com/?apikey={}&i={}", token, title);
    let url = Uri::from_str(&omdb_url);

    if let Err(err) = url {
        return Err(format!("Broken IMDB url {}", err));
    }

    let imdb_response = client.get(url.unwrap()).await;
    if let Err(err) = imdb_response {
        return Err(format!("OMDB error: {}", err));
    }

    let body: Body = imdb_response.unwrap().into_body();
    let body = to_bytes(body).await;

    if let Err(err) = body {
        return Err(format!("Error {}", err));
    }

    let new_body = body.unwrap();
    let str = String::from_utf8_lossy(&new_body);
    let v = serde_json::from_str(&str);

    if let Err(err) = v {
        return Err(format!("{}", err));
    }

    let formatted_body: OmdbData = v.unwrap();
    let response = formatted_body.clone().response;

    if response.is_none() {
        return Err(format!(
            "OMDB didn't include a Response in the response {:?}",
            formatted_body
        ));
    }

    match response.unwrap().as_ref() {
        "True" => Ok(format!(
            "{} ({})",
            formatted_body.title.unwrap(),
            formatted_body.year.unwrap()
        )),
        "False" => Ok(formatted_body.error.unwrap().to_string()),
        _ => Ok(String::from(str)),
    }
}
