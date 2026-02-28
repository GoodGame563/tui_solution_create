use crate::structs::ConfigFileCreation as Config;
use crate::structs::{AppConfig, LeetCodeProblem};
use crate::utils::get_config_from_path;
use anyhow::Result;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn create_project(project_name: &str, path: &str, app_config: &AppConfig) -> Result<()> {
    let config: Config = get_config_from_path(path)?;
    let base_path = format!("{}/{}", app_config.solutions_url, project_name);
    let base = Path::new(&base_path);
    fs::create_dir_all(base)?;
    for f in &config.folders {
        fs::create_dir_all(base.join(f))?;
    }
    for f in &config.files {
        let content = f.content.replace("{{name}}", project_name);
        let path = base.join(&f.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
    }

    for cmd in &config.commands {
        if cmd.is_empty() {
            continue;
        }
        let processed: Vec<String> = cmd
            .iter()
            .map(|s| s.replace("{{name}}", project_name))
            .collect();

        let mut c = Command::new(&processed[0]);
        c.args(&processed[1..]);
        c.current_dir(base);

        let _ = c.status();
    }
    match app_config.create_local_gitignore.mode {
        crate::structs::ToggleMode::No => return Ok(()),
        crate::structs::ToggleMode::Yes => {}
        crate::structs::ToggleMode::YesSome => for el in &app_config.create_local_gitignore.list {},
    };
    Ok(())
}

fn create_readme(problem: &LeetCodeProblem) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let filename = format!("README.md");
    let mut file = File::create(&filename)?;

    let plain_description = strip_html_tags(&problem.description);

    let content = format!(
        r#"# {}

## Information

- **Difficult:** {}
- **Likes:** {}
- **Dislikes:** {}

## Description

{}

```
"#,
        problem.title, problem.difficulty, problem.likes, problem.dislikes, plain_description,
    );

    file.write_all(content.as_bytes())?;
    println!("\nREADME создан: {}", filename);

    Ok(())
}

fn strip_html_tags(html: &str) -> String {
    let mut result = html.to_string();

    let re_img = Regex::new(r"<img[^>]*>").unwrap();
    result = re_img.replace_all(&result, "").to_string();

    let re_strong = Regex::new(r"</?(?:strong|b)>").unwrap();
    result = re_strong.replace_all(&result, "").to_string();

    let re_code = Regex::new(r"<code>([^<]*)</code>").unwrap();
    result = re_code.replace_all(&result, "`$1`").to_string();

    let re_em = Regex::new(r"</?(?:em|i)>").unwrap();
    result = re_em.replace_all(&result, "").to_string();

    let re_tags = Regex::new(r"<[^>]+>").unwrap();
    result = re_tags.replace_all(&result, "").to_string();

    result = result
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    let mut prev_empty = false;
    let mut cleaned = String::new();
    for line in result.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_empty {
                cleaned.push('\n');
                prev_empty = true;
            }
        } else {
            cleaned.push_str(line);
            cleaned.push('\n');
            prev_empty = false;
        }
    }

    cleaned.trim().to_string()
}
