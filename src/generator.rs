use crate::structs::ConfigFileCreation as Config;
use crate::structs::AppConfig;
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn create_project(project_name: &str, language: &str, app_config: &AppConfig) -> Result<()>{
    let yaml = fs::read_to_string(format!("{}/{}.yaml", app_config.recipes_url, language))?;
    let config: Config = serde_yaml::from_str(&yaml)?;
    let base_path = format!("{}/{}", app_config.solutions_url, project_name);
    let base = Path::new(&base_path);
    fs::create_dir_all(base)?;
    for f in &config.folders {
        fs::create_dir_all(base.join(f))?;
        println!("  📁 {}", f);
    }
    for f in &config.files {
        let content = f.content.replace("{{name}}", project_name);
        let path = base.join(&f.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
        println!("  📄 {}", f.path);
    }

    for cmd in &config.commands {
        if cmd.is_empty() { continue; }
        let processed: Vec<String> = cmd.iter()
            .map(|s| s.replace("{{name}}", project_name))
            .collect();

        let mut c = Command::new(&processed[0]);
        c.args(&processed[1..]);
        c.current_dir(base);

        println!("  ⚡ {}", processed.join(" "));
        let _ = c.status();
    }
    Ok(())
}