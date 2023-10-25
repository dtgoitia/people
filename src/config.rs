use expanduser::expanduser;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use tracing::{debug, info};

use crate::model::Person;

const CONFIG_PATH: &str = ".config/people/config.yaml";

#[derive(Debug, Clone)]
pub struct Config {
    pub people_dir: PathBuf,
    pub ignore: Vec<Person>,
}

impl Config {
    pub fn get_per_person_dir(&self) -> PathBuf {
        self.people_dir.join("per-person-logs")
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct ConfigFile {
    pub people_dir: Box<PathBuf>,
    pub ignore: Option<Vec<Person>>,
}

type ErrorReason = String;

#[derive(Debug)]
pub enum ConfigError {
    HomeNotFound,
    ConfigFileNotFound(PathBuf),
    ConfigFileHasUnsupportedFormat(ErrorReason),
}

fn parse_config(content: String) -> Result<ConfigFile, String> {
    match serde_yaml::from_str::<ConfigFile>(&content) {
        Ok(config_file) => Ok(config_file),
        Err(error) => {
            debug!("failed to parse config file, reason: {error:?}");
            return Err(error.to_string());
        }
    }
}

fn load_config_from_user_config_file() -> Result<ConfigFile, ConfigError> {
    let home_str = match std::env::var("HOME") {
        Ok(home) => home,
        Err(error) => {
            debug!("could not find HOME environment variable, reason: {error:?}");
            return Err(ConfigError::HomeNotFound);
        }
    };

    let home = Path::new(&home_str);
    let path = home.join(CONFIG_PATH.to_string());

    if path.exists() == false {
        return Err(ConfigError::ConfigFileNotFound(path));
    }

    let content = fs::read_to_string(&path).unwrap();

    match parse_config(content) {
        Ok(config_file) => Ok(config_file),
        Err(error) => {
            return Err(ConfigError::ConfigFileHasUnsupportedFormat(
                error.to_string(),
            ));
        }
    }
}

pub fn get_config() -> Result<Config, String> {
    let config_file = match load_config_from_user_config_file() {
        Ok(config) => config,
        Err(reason) => {
            let reason = match reason {
                ConfigError::HomeNotFound => format!("HOME not found"),
                ConfigError::ConfigFileNotFound(expected_path) => {
                    format!("expected file at {expected_path:?}, but it does not exist")
                }
                ConfigError::ConfigFileHasUnsupportedFormat(parse_failure) => {
                    format!("failed to parse because {parse_failure}")
                }
            };
            info!("config file not loaded, reason: {reason}");
            return Err(reason);
        }
    };

    let ignore: Vec<Person> = match config_file.ignore {
        Some(people) => people,
        None => vec![],
    };

    let people_dir = match expanduser(config_file.people_dir.display().to_string()) {
        Ok(path) => path,
        Err(reason) => return Err(reason.to_string()),
    };

    let config = Config { people_dir, ignore };

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_parse_config_with_ignore() {
        let config_file_content = r#"
        people_dir: ~/people
        ignore:
          - JohnDoe
          - JaneDoe
        "#
        .to_string();

        let expected = Ok(ConfigFile {
            people_dir: Box::new(Path::new("~/people").to_path_buf()),
            ignore: Some(vec!["JohnDoe".to_string(), "JaneDoe".to_string()]),
        });

        assert_eq!(parse_config(config_file_content), expected);
    }

    #[test]
    fn test_parse_config_without_ignore() {
        let config_file_content = r#"
        people_dir: ~/people
        "#
        .to_string();

        let expected = Ok(ConfigFile {
            people_dir: Box::new(Path::new("~/people").to_path_buf()),
            ignore: None,
        });

        assert_eq!(parse_config(config_file_content), expected);
    }

    #[test]
    fn test_parse_config_with_special_characters() {
        let config_file_content = r#"
        people_dir: ~/people
        ignore:
          - Lucía
        "#
        .to_string();

        let expected = Ok(ConfigFile {
            people_dir: Box::new(Path::new("~/people").to_path_buf()),
            ignore: Some(vec!["Lucía".to_string()]),
        });

        assert_eq!(parse_config(config_file_content), expected);
    }
}
