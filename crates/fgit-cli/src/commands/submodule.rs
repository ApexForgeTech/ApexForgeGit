use std::env;
use std::fs;
use std::path::Path;
use colored::Colorize;
use fgit_core::Repository;
use crate::commands::clone;

pub fn add(url: &str, path: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let abs_path = repo.workdir.join(path);
    if abs_path.exists() {
        return Err(format!("Destination path '{}' already exists", path));
    }

    println!("  {} Adding submodule '{}' at '{}'...", "ℹ".bright_blue(), url.bright_cyan(), path.bright_yellow());

    // Call clone logic securely (it creates .fgit in abs_path)
    clone::run(url, Some(path))?;

    // Update .fgitmodules file
    let modules_file = repo.workdir.join(".fgitmodules");
    let mut config = if modules_file.exists() {
        fs::read_to_string(&modules_file).unwrap_or_default()
    } else {
        String::new()
    };

    let entry = format!("\n[submodule \"{}\"]\n\tpath = {}\n\turl = {}\n", path, path, url);
    config.push_str(&entry);

    fs::write(modules_file, config).map_err(|e| e.to_string())?;

    println!("  {} Submodule registered successfully in .fgitmodules", "✓".bright_green());
    Ok(())
}

pub fn update() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let modules_file = repo.workdir.join(".fgitmodules");
    if !modules_file.exists() {
        println!("  {} No .fgitmodules found in working directory", "ℹ".bright_blue());
        return Ok(());
    }

    let config = fs::read_to_string(&modules_file).map_err(|e| e.to_string())?;
    
    // Very dummy parser for MVP
    let mut current_path = None;
    let mut current_url = None;

    for line in config.lines() {
        let line = line.trim();
        if line.starts_with("path =") {
            current_path = Some(line.replace("path =", "").trim().to_string());
        } else if line.starts_with("url =") {
            current_url = Some(line.replace("url =", "").trim().to_string());
        }

        if let (Some(ref p), Some(ref u)) = (&current_path, &current_url) {
            let abs_path = repo.workdir.join(p);
            if !abs_path.exists() || !abs_path.join(".fgit").exists() {
                println!("  {} Cloning missing submodule '{}'...", "ℹ".bright_blue(), p);
                if let Err(e) = clone::run(u, Some(p)) {
                    println!("  {} Failed to clone '{}': {}", "✗".bright_red(), p, e);
                }
            } else {
                println!("  {} Submodule '{}' is already initialized", "✓".bright_green(), p);
            }
            current_path = None;
            current_url = None;
        }
    }

    println!("  {} Submodules update complete", "✓".bright_green());
    Ok(())
}
