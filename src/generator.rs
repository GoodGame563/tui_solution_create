use crate::structs::ConfigFileCreation as Config;
use crate::structs::ToggleMode::{No, Yes, YesSome};
use crate::structs::{AppConfig, LeetCodeProblem};
use regex::Regex;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub fn create_project(
    project_name: &str,
    config: &Config,
    app_config: &AppConfig,
    problem: &LeetCodeProblem,
) -> Result<(), String> {
    let base_path = format!(
        "{}/{}/{}",
        app_config.solutions_url, config.name, project_name
    );
    let base = Path::new(&base_path);
    fs::create_dir_all(base).map_err(|e| format!("Error creating directory: {}", e))?;
    for f in &config.folders {
        fs::create_dir_all(base.join(f))
            .map_err(|e| format!("Error creating folder {}: {}", f, e))?;
    }
    for f in &config.files {
        let content = f.content.replace("{{name}}", project_name);
        let path = base.join(&f.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Error creating parent directory: {}", e))?;
        }
        fs::write(&path, content)
            .map_err(|e| format!("Error writing file {}: {}", path.display(), e))?;
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
        c.stdout(std::process::Stdio::null());
        c.stderr(std::process::Stdio::null());

        let _ = c.output();
    }
    create_readme(problem, &base_path)?;

    let should_create_gitignore = match &app_config.create_local_gitignore.mode {
        No => false,
        Yes => true,
        YesSome => app_config
            .create_local_gitignore
            .list
            .iter()
            .any(|el| *el == config.name),
    };

    if should_create_gitignore {
        if let Some(g) = &config.gitignore {
            create_gitignore(&base_path, g)?;
        }
    }

    let should_init_git = match &app_config.init_git.mode {
        No => false,
        Yes => true,
        YesSome => app_config.init_git.list.iter().any(|el| *el == config.name),
    };

    if should_init_git {
        let mut git_init = Command::new("git");
        git_init.arg("init").current_dir(base);
        let _ = git_init.output();
    }

    let should_open_terminal = match &app_config.open_terminal.mode {
        No => false,
        Yes => true,
        YesSome => app_config
            .open_terminal
            .list
            .iter()
            .any(|el| *el == config.name),
    };

    if should_open_terminal {
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd")
                .arg("/C")
                .arg("start")
                .arg("cmd")
                .arg("/K")
                .arg(format!("cd {}", base_path))
                .spawn();
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = Command::new("gnome-terminal")
                .arg("--working-directory")
                .arg(base_path)
                .spawn();
        }
    }

    Ok(())
}

fn create_gitignore(path: &str, ignore: &str) -> Result<(), String> {
    let filename = format!(".gitignore");
    let mut file = File::create(format!("{}/{}", path, filename))
        .map_err(|e| format!("Error creating README: {}", e))?;
    file.write_all(ignore.as_bytes())
        .map_err(|e| format!("Error writing README: {}", e))?;
    Ok(())
}

fn create_readme(problem: &LeetCodeProblem, path: &str) -> Result<(), String> {
    let filename = format!("README.md");
    let mut file = File::create(format!("{}/{}", path, filename))
        .map_err(|e| format!("Error creating README: {}", e))?;

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

    file.write_all(content.as_bytes())
        .map_err(|e| format!("Error writing README: {}", e))?;

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
