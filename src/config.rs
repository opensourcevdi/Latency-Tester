use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    #[serde(default = "default_address")]
    pub address: String,
}

fn default_address() -> String {
    "127.0.0.1:4999".to_string()
}

pub fn read_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let mut data = fs::read(path);
    if matches!(data,Err(_)) {
        fs::write(path, "").expect(format!("error creating config file {:?}", path).as_str());
        data = fs::read(path);
    }
    let data = data.unwrap();
    let text = String::from_utf8(data)?;
    let config: Config = toml::from_str(&text)?;
    Ok(config)
}

pub fn write_config(config: &Config, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let text = toml::to_string(config)?;
    fs::write(path, text)?;
    Ok(())
}