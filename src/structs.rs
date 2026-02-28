use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Default, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ToggleMode {
    #[default]
    No,
    Yes,
    YesSome,
}

#[derive(Clone, Deserialize, Default, Serialize)]
pub struct ToggleWithList {
    pub mode: ToggleMode,
    pub list: Vec<String>,
    pub list_input: String,
    pub list_input_active: bool,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AppConfig {
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

#[derive(Deserialize, Clone, Default)]
pub struct ConfigFileCreation {
    pub name: String,
    pub folders: Vec<String>,
    pub files: Vec<FileEntry>,
    pub commands: Vec<Vec<String>>,
    pub color: Option<String>,
    pub gitignore: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct FileEntry {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LeetCodeProblem {
    pub title: String,
    pub difficulty: String,
    pub description: String,
    pub example_testcases: String,
    pub likes: u64,
    pub dislikes: u64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct AllSettings {
    pub recipes_url: String,
    pub solutions_url: String,
    pub init_git: ToggleWithList,
    pub create_local_gitignore: ToggleWithList,
    pub open_terminal: ToggleWithList,
    pub open_ide: ToggleWithList,
}

impl AllSettings {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            recipes_url: config.recipes_url.clone(),
            solutions_url: config.solutions_url.clone(),
            init_git: config.init_git.clone(),
            create_local_gitignore: config.create_local_gitignore.clone(),
            open_terminal: config.open_terminal.clone(),
            open_ide: config.open_ide.clone(),
        }
    }

    pub fn to_config(&self) -> AppConfig {
        AppConfig {
            recipes_url: self.recipes_url.clone(),
            solutions_url: self.solutions_url.clone(),
            init_git: self.init_git.clone(),
            create_local_gitignore: self.create_local_gitignore.clone(),
            open_terminal: self.open_terminal.clone(),
            open_ide: self.open_ide.clone(),
        }
    }
}

pub struct Project {
    pub url: String,
    pub name: String,
    pub recipes_name: String,
}
