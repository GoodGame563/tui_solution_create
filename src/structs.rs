use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToggleMode {
    No,
    #[default]
    Yes,
    YesSome,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct ToggleWithList {
    pub mode: ToggleMode,
    pub list: Vec<String>,
    pub list_input: String,
    pub list_input_active: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    #[serde(default)]
    pub init_git: ToggleWithList,
    #[serde(default)]
    pub create_local_gitignore: ToggleWithList,
    #[serde(default)]
    pub recipes_url: String,
    #[serde(default)]
    pub solutions_url: String,
    #[serde(default)]
    pub open_terminal: ToggleWithList,
    #[serde(default)]
    pub open_ide: ToggleWithList,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            init_git: ToggleWithList::default(),
            create_local_gitignore: ToggleWithList::default(),
            recipes_url: String::from("recipes"),
            solutions_url: String::from("solutions"),
            open_terminal: ToggleWithList::default(),
            open_ide: ToggleWithList::default(),
        }
    }
}

#[derive(Deserialize)]
pub struct ConfigFileCreation {
    #[serde(default)]
    pub name: String,
    pub folders: Vec<String>,
    pub files: Vec<FileEntry>,
    #[serde(default)]
    pub commands: Vec<Vec<String>>
}

#[derive(Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub content: String,
}