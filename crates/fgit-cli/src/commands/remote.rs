use fgit_core::Repository;
use colored::Colorize;
use std::env;

pub fn add(name: &str, url: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let mut config = repo.config().map_err(|e| e.to_string())?;
    config.add_remote(name.to_string(), url.to_string());
    repo.save_config(&config).map_err(|e| e.to_string())?;
    println!("  {} Remote '{}' added → {}",
        "✓".bright_green(), name.bright_cyan(), url.dimmed());
    Ok(())
}

pub fn list() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let config = repo.config().map_err(|e| e.to_string())?;

    println!();
    if config.remote.is_empty() {
        println!("  {} {}", "ℹ".bright_blue(), "No remotes configured.".dimmed());
    } else {
        for (name, remote) in &config.remote {
            println!("  {} {} → {}", "⬡".bright_cyan(), name.bright_white(), remote.url.dimmed());
        }
    }
    println!();
    Ok(())
}

pub fn remove(name: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let mut config = repo.config().map_err(|e| e.to_string())?;
    if !config.remove_remote(name) {
        return Err(format!("Remote '{}' not found", name));
    }
    repo.save_config(&config).map_err(|e| e.to_string())?;
    println!("  {} Remote '{}' removed", "✓".bright_green(), name.bright_red());
    Ok(())
}
