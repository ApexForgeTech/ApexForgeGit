use fgit_core::Repository;
use fgit_core::stash::StashManager;
use colored::Colorize;
use std::env;

pub fn save() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let mgr = StashManager::new(&repo.fgit_dir);
    let id = mgr.save("WIP", vec![])?;
    println!("  {} Saved working directory to stash @{}",
        "✓".bright_green(), id.to_string().bright_yellow());
    Ok(())
}

pub fn pop() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let mgr = StashManager::new(&repo.fgit_dir);
    let entry = mgr.pop()?;
    println!("  {} Restored stash @{}: {}",
        "✓".bright_green(),
        entry.id.to_string().bright_yellow(),
        entry.message.dimmed());
    Ok(())
}

pub fn list() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let mgr = StashManager::new(&repo.fgit_dir);
    let entries = mgr.list()?;

    println!();
    if entries.is_empty() {
        println!("  {} {}", "ℹ".bright_blue(), "No stash entries.".dimmed());
    } else {
        for e in &entries {
            println!("  {} stash@{}: {} ({})",
                "◆".bright_magenta(),
                e.id.to_string().bright_yellow(),
                e.message.bright_white(),
                e.timestamp.format("%Y-%m-%d %H:%M").to_string().dimmed());
        }
    }
    println!();
    Ok(())
}
