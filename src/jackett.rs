fn jackett_host() -> String {
    match env::var("JACKETT_HOST") {
        Ok(host) => host,
        Err(_) => String::from("http://localhost:9117"),
    }
}

fn jackett_token() -> Result<String, String> {
    match env::var("JACKETT_TOKEN") {
        Ok(token) => Ok(token),
        Err(_) => match env::var("JACKETT_DATA_DIR") {
            Ok(data_dir) => {
                let file_name = data_dir + "/ServerConfig.json";

                let file_content = fs::read_to_string(file_name);

                let v: serde_json::Value =
                    serde_json::from_str(file_content.unwrap().as_str()).unwrap();

                if v["APIKey"] == serde_json::Value::Null {
                    Err(String::from("Jackett file does not have key APIKey"))
                } else {
                    Ok(v["APIKey"].as_str().unwrap().to_string())
                }
            }
            Err(_) => Err(String::from(
                "Set JACKETT_TOKEN or JACKETT_DATA_DIR if jackett is in the same host",
            )),
        },
    }
}

#[derive(serde::Deserialize)]
struct Indexer {
    #[serde(rename(deserialize = "Name"))]
    name: String,
}

#[derive(serde::Deserialize, Clone)]
struct Torrent {
    #[serde(rename(deserialize = "Seeders"))]
    seeders: i64,
    #[serde(rename(deserialize = "Peers"))]
    peers: i64,
    #[serde(rename(deserialize = "MagnetUri"))]
    magnet_uri: String,
    #[serde(rename(deserialize = "Title"))]
    title: String,
    #[serde(rename(deserialize = "Category"))]
    categories: Vec<i64>,
    #[serde(rename(deserialize = "Details"))]
    detail_url: String,
    #[serde(rename(deserialize = "Size"))]
    size: u64,
}

#[derive(serde::Deserialize)]
struct JackettResponse {
    #[serde(rename(deserialize = "Indexers"))]
    indexers: Vec<Indexer>,
    #[serde(rename(deserialize = "Results"))]
    results: Vec<Torrent>,
}

#[derive(Clone)]
struct TelegramJackettResponse {
    torrents: Vec<Torrent>,
}

async fn request_jackett(
    query_string: String,
    client: client::Client<hyper_rustls::HttpsConnector<client::HttpConnector>>,
) -> Result<TelegramJackettResponse, String> {
    let encoded_path: String = form_urlencoded::Serializer::new(String::new())
        .append_pair("apikey", jackett_token().unwrap().as_str())
        .append_pair("Query", query_string.as_str())
        .finish();

    let url = [
        jackett_host(),
        String::from("/api/v2.0/indexers/all/results?"),
        encoded_path,
    ]
    .join("");

    let res = client.get(Uri::from_str(&url).unwrap()).await.unwrap();
    let body: Body = res.into_body();
    let body = to_bytes(body).await.unwrap();
    let str = String::from_utf8_lossy(&body);

    let mut v: JackettResponse = serde_json::from_str(&str).unwrap();
    if v.indexers.len() == 0 && v.results.len() == 0 {
        return Err("Add some indexers".to_string());
    }

    v.results.sort_by_key(|d1| -d1.seeders);
    let torrents = v.results.into_iter().take(20).collect();

    let response = TelegramJackettResponse { torrents };

    return Ok(response);
}

async fn dispatch_search(
    api: Api,
    message: Message,
    response: TelegramJackettResponse,
) -> Result<(), Error> {
    let info = format_torrent(response);

    let formatted_text = format!("<pre>{}</pre>", info);

    let mut reply = message.text_reply(formatted_text);

    api.send(reply.parse_mode(ParseMode::Html)).await?;

    Ok(())
}

fn format_torrent(response: TelegramJackettResponse) -> String {
    return response
        .torrents
        .iter()
        .enumerate()
        .fold(String::from(""), |text, (i, t)| {
            text + format!(
                "{}. {} - {}B - {}\n",
                i + 1,
                t.title,
                SizeFormatterSI::new(t.size),
                t.seeders
            )
            .as_str()
        });
}


fn is_movie(categories: Vec<i64>) -> bool {
    return categories.iter().any(|c| c >= &2000 && c < &3000);
}

fn is_tv_show(categories: Vec<i64>) -> bool {
    return categories.iter().any(|c| c >= &3000 && c < &4000);
}


async fn dispatch_from_reply(
    api: Api,
    message: Message,
    index: u16,
    torrents: Vec<TelegramJackettResponse>,
) -> Result<(), Error> {
    let reply_text = message.reply_to_message.unwrap().text().unwrap();
    let real_index = index - 1;

    let jackett = torrents.clone().into_iter().find(|response| {
        format_torrent(response.clone())
            .split_whitespace()
            .collect::<String>()
            == reply_text.split_whitespace().collect::<String>()
    });

    match jackett {
        Some(jackett) => {
            let torrent = jackett.torrents.iter().nth(real_index.into()).unwrap();
            println!("{:?}", is_tv_show(torrent.clone().categories));

            //
            // TODO: send magnet_url to flexget
        },
        None => {
            return Ok(());
        }
    }

    // let torrent = jackett.unwrap().torrents.iter().nth(real_index.into()).unwrap();

    Ok(())
}
