use crate::structs::ConfigFileCreation as Config;
use dict::{Dict, DictIface};
use std::error::Error;
use std::{fmt, fs};

#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    YamlError(serde_yaml::Error),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::IoError(err) => write!(f, "Error read file: {}", err),
            ConfigError::YamlError(err) => write!(f, "Error parsing YAML: {}", err),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::IoError(err) => Some(err),
            ConfigError::YamlError(err) => Some(err),
        }
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigError::YamlError(err)
    }
}

pub fn get_config_from_path(path: &str) -> Result<Config, ConfigError> {
    let yaml = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&yaml)?)
}

pub fn get_all_config(dir: &str) -> Vec<Config> {
    let mut configs: Vec<Config> = vec![];
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path().to_str().unwrap_or("lol").to_string();
            let config = match get_config_from_path(&path) {
                Ok(c) => c,
                Err(_) => {
                    continue;
                }
            };
            configs.push(config);
        }
    }
    configs
}

pub fn get_dict_from_dir(dir: &str) -> Dict<String> {
    let mut dict = Dict::<String>::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path().to_str().unwrap_or("lol").to_string();
            let config = match get_config_from_path(&path) {
                Ok(c) => c,
                Err(_) => {
                    continue;
                }
            };
            dict.add(config.name, path);
        }
    }
    dict
}
