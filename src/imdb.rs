use hyper::{body::to_bytes, client, Body, Uri};
use std::env;
use std::str::FromStr;

fn omdb_token() -> String {
    match env::var("OMDB_TOKEN") {
        Ok(token) => token,
        Err(_) => String::new(),
    }
}

fn imdb_title(imdb_url: String) -> Result<String, String> {
    let url = imdb_url.parse::<Uri>();

    match url {
        Ok(url) => {
            let fragments = url
                .path()
                .split("/")
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>();

            let imdb_title = fragments.last();

            match imdb_title {
                Some(title) => Ok(title.to_string()),
                None => Err("Couldn't find the imdb id from the url".to_string())
            }
        }
        Err(err) => {
            println!("{}", err);
            Err("Undefined URL".to_string())
        }
    }
}

#[derive(serde::Deserialize)]
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
    let https = hyper_rustls::HttpsConnector::new();
    let client: client::Client<_, hyper::Body> = client::Client::builder().build(https);

    let token = omdb_token();
    if token.is_empty() {
        return Err("OMDB Token is not configured".to_string());
    }

    let title = imdb_title(imdb_url)?;

    let omdb_url = format!("http://www.omdbapi.com/?apikey={}&i={}", token, title);
    let res = client.get(Uri::from_str(&omdb_url).unwrap()).await.unwrap();

    let body: Body = res.into_body();
    let body = to_bytes(body).await.unwrap();

    let str = String::from_utf8_lossy(&body);
    let v: OmdbData = serde_json::from_str(&str).unwrap();
    match v.response.unwrap().as_ref() {
        "True" => {
            return Ok(format!("{} ({})", v.title.unwrap(), v.year.unwrap()));
        }
        "False" => {
            return Ok(v.error.unwrap().to_string());
        }
        _ => {
            return Ok(String::from(str));
        }
    }
}
