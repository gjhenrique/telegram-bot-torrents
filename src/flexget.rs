use std::env;
use std::process::Command;

fn flexget_path() -> Result<String, String> {
    match env::var("FLEXGET_PATH") {
        Ok(host) => Ok(host),
        Err(_) => Err("FLEXGET_PATH not configured".to_string()),
    }
}

pub enum Media {
    TV,
    Movie
}


pub fn execute_magnet_url(magnet_url: String, media: Media) -> Result<(), String> {
    let command = match media {
        Media::TV => "download-tv-manual",
        Media::Movie => "download-movie-manual"
    };

    let command = format!("--task {} --cli-config 'magnet={}'", command, magnet_url);

    flexget_command(command)?;

    Ok(())
}

pub fn sync_flexget() -> Result<(), String> {
    flexget_command("--no-cache --discover-now".to_string())?;

    Ok(())
}

fn flexget_command(flexget_command: String) -> Result<(), String> {
    let path = flexget_path()?;

    let command = Command::new("sh")
        .arg("-c")
        .arg(format!("flexget execute {}", flexget_command))
        .current_dir(path)
        .output();

    match command {
        Ok(c) => {
            if c.status.success() {
                return Ok(());
            } else {
                println!("Stderr: {}", String::from_utf8_lossy(&c.stderr));

                let stderr = String::from_utf8(c.stderr);

                if let Ok(stderr) = stderr {
                    return Err(stderr);
                } else {
                    return Err("Error when converting stderr\nCheck logs".to_string());
                }
            }
        }
        Err(err) => {
            println!("Error: {}", err.to_string());
            return Err(err.to_string());
        }
    }
}
