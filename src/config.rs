use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub shell: String,
    pub run: Run,
    pub backup: Backup,
}

#[derive(Debug, Deserialize, Default)]
pub struct Run {
    pub run_commands: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Backup {
    pub cloud_init_commands: Vec<String>,
    pub cloud_commit_commands: Vec<String>,
    pub cloud_push_commands: Vec<String>,
}
