use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub shell: String,
    pub run: Run,
    pub backup: Backup,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            shell: String::from("bash"),
            run: Default::default(),
            backup: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct Run {
    pub commands: Vec<String>,
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Backup {
    #[serde(rename(deserialize = "cloudInitCommands"))]
    pub cloud_init_commands: Vec<String>,
    #[serde(rename(deserialize = "cloudCommitCommands"))]
    pub cloud_commit_commands: Vec<String>,
    #[serde(rename(deserialize = "cloudPushCommands"))]
    pub cloud_push_commands: Vec<String>,
}
