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
