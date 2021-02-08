use std::env;
use std::process::Command;

fn flexget_path() -> Option<String> {
    match env::var("FLEXGET_PATH") {
        Ok(host) => Some(host),
        Err(_) => None,
    }
}

fn flexget_command(flexget_command: String) -> Result<(), &'static str> {
    let path = flexget_path().unwrap();

    let command = Command::new("sh")
        .arg("-c")
        .arg(format!("lslsls execute {}", flexget_command))
        .current_dir(path)
        .output();

    match command {
        Ok(c) => {
            if c.status.success() {
                return Ok(());
            } else {
                println!("Stderr: {}", String::from_utf8_lossy(&c.stderr));
                return Err("Error command-line");
            }
            // println!("Stdout: {}", String::from_utf8_lossy(&c.stdout));
        }
        Err(err) => {
            println!("Error: {}", err.to_string());
            return Err("Error");
        }
    }
}

pub fn sync_flexget() -> Result<(), &'static str> {
    return flexget_command("--no-cache --discover-now".to_string());
}

pub enum Media {
    TV,
    Movie
}

pub fn execute_magnet_url(magnet_url: String, media: Media) -> Result<(), &'static str> {
    let command = match media {
        TV => "download-tv-manual",
        Movie => "download-movie-manual"
    };

    let command = format!("--task {} --cli-config 'magnet={{}}'", command);
    // TODO: Add if tv else movie

    // "--task download-movie-manual --cli-config \"magnet={}\"",
    let argument = format!(
        "--task  --cli-config 'magnet={}'",
        magnet_url
    );

    return flexget_command(argument.to_string());
}
