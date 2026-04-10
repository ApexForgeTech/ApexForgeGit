use fgit_core::Repository;
use colored::Colorize;
use std::env;

pub fn run(name: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let refs = repo.refs();

    // Verify branch exists
    let target_hash = refs.resolve_branch(name)?
        .ok_or_else(|| format!("Branch '{}' does not exist", name))?;

    // Update HEAD
    refs.set_head_to_branch(name)?;

    println!("  {} Switched to branch '{}'",
        "✓".bright_green(), name.bright_cyan());
    println!("  {} at commit {}",
        "→".bright_yellow(), &target_hash[..8].bright_yellow());

    Ok(())
}
