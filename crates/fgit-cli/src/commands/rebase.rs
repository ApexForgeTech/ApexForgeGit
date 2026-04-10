use colored::Colorize;
use fgit_core::Repository;
use std::env;

pub fn run(target_branch: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let refs = repo.refs();
    let current_branch = refs.current_branch().map_err(|e| e.to_string())?
        .ok_or_else(|| "Cannot rebase in detached HEAD state".to_string())?;

    if current_branch == target_branch {
        println!("  {} Already on '{}'", "ℹ".bright_blue(), target_branch);
        return Ok(());
    }

    let target_hash = refs.resolve_branch(target_branch)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Branch '{}' not found", target_branch))?;

    let current_hash = refs.resolve_branch(&current_branch)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Current branch '{}' not found", current_branch))?;

    println!("  {} Rebasing {} onto {}...", "ℹ".bright_blue(), current_branch.bright_cyan(), target_branch.bright_white());

    if target_hash == current_hash {
        println!("  {} Already up to date.", "✓".bright_green());
        return Ok(());
    }

    // In a full implementation, we perform cherry-picks or 3-way merges over a range.
    // For this MVP, we simulate a fast-forward if possible. Since we don't have a 
    // full commit graph traverser implemented in core, we will just use a fast-forward placeholder
    // or fail mentioning linear rewrite requirements.
    
    println!("  {} {}", "⚠".bright_yellow(), "Linear history rewrite (rebase) requires interactive resolution.".dimmed());
    println!("  {} {}", "✗".bright_red(), "Non-fast-forward rebase is not fully supported in mvp. Use `fgit merge` instead.".yellow());

    Ok(())
}
