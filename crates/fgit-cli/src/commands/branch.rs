use fgit_core::Repository;
use colored::Colorize;
use std::env;

pub fn create(name: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let refs = repo.refs();

    let current_hash = refs.resolve_head().map_err(|e| e)?
        .unwrap_or_else(|| "0".repeat(64));

    refs.create_branch(name, &current_hash)?;
    println!("  {} Branch '{}' created at {}",
        "✓".bright_green(), name.bright_cyan(), &current_hash[..8].bright_yellow());
    Ok(())
}

pub fn delete(name: &str, force: bool) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let refs = repo.refs();
    // Simulate force checks
    if force {
        println!("  {} Force deleting branch...", "⚠".bright_yellow());
    }
    refs.delete_branch(name)?;
    println!("  {} Branch '{}' deleted", "✓".bright_green(), name.bright_red());
    Ok(())
}

pub fn list() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let refs = repo.refs();

    let current = refs.current_branch().unwrap_or(None);
    let branches = refs.list_branches()?;

    println!();
    if branches.is_empty() {
        println!("  {} {}", "ℹ".bright_blue(), "No branches yet.".dimmed());
    } else {
        for b in &branches {
            if current.as_deref() == Some(b.as_str()) {
                println!("  {} {}", "●".bright_green(), b.bright_green().bold());
            } else {
                println!("  {} {}", "○".dimmed(), b);
            }
        }
    }
    println!();
    Ok(())
}
