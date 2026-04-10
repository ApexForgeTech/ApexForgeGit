use fgit_core::Repository;
use colored::Colorize;
use std::env;
use std::fs;

pub fn run(name: Option<&str>, message: Option<&str>) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let refs = repo.refs();

    match name {
        None => {
            // List tags
            let tags_dir = repo.fgit_dir.join("refs/tags");
            println!();
            if !tags_dir.exists() {
                println!("  {} {}", "ℹ".bright_blue(), "No tags.".dimmed());
            } else {
                let entries = fs::read_dir(&tags_dir).map_err(|e| format!("{}", e))?;
                let mut tags: Vec<String> = Vec::new();
                for e in entries {
                    if let Ok(e) = e {
                        tags.push(e.file_name().to_str().unwrap_or("").to_string());
                    }
                }
                tags.sort();
                if tags.is_empty() {
                    println!("  {} {}", "ℹ".bright_blue(), "No tags.".dimmed());
                } else {
                    for t in &tags {
                        println!("  {} {}", "🏷".to_string(), t.bright_yellow());
                    }
                }
            }
            println!();
        }
        Some(tag_name) => {
            let target = refs.resolve_head()?
                .ok_or("No commits to tag")?;

            let tag_path = repo.fgit_dir.join("refs/tags").join(tag_name);
            if tag_path.exists() {
                return Err(format!("Tag '{}' already exists", tag_name));
            }

            fs::write(&tag_path, format!("{}\n", target))
                .map_err(|e| format!("Failed to create tag: {}", e))?;

            if let Some(msg) = message {
                println!("  {} Created annotated tag '{}' at {} — \"{}\"",
                    "✓".bright_green(), tag_name.bright_yellow(),
                    &target[..8].bright_cyan(), msg);
            } else {
                println!("  {} Created tag '{}' at {}",
                    "✓".bright_green(), tag_name.bright_yellow(),
                    &target[..8].bright_cyan());
            }
        }
    }

    Ok(())
}
