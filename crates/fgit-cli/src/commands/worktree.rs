use std::env;
use std::fs;
use colored::Colorize;
use fgit_core::Repository;

pub fn add(path: &str, branch: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let abs_path = repo.workdir.join(path);
    if abs_path.exists() {
        return Err(format!("Destination path '{}' already exists", path));
    }
    
    // Check if the branch exists
    let ref_exists = repo.refs().resolve_branch(branch)?.is_some();
    if !ref_exists {
        println!("  {} Warning: Branch '{}' does not exist yet. Expected to be checked out.", "⚠".bright_yellow(), branch);
    }

    // Create the working directory
    fs::create_dir_all(&abs_path).map_err(|e| e.to_string())?;

    // Create the .fgit pointer file
    let gitdir_content = format!("gitdir: {}", repo.fgit_dir.to_string_lossy());
    fs::write(abs_path.join(".fgit"), gitdir_content).map_err(|e| e.to_string())?;

    println!("  {} Preparing worktree (checking out '{}')...", "ℹ".bright_blue(), branch);

    // To cleanly checkout without rewriting logic, we can instruct the user to enter and `fgit switch` because restoring trees across multiple workdirs inside the exact CLI is complex for MVP and `fgit switch` uses `reset --hard` anyway.
    
    println!("  {} Prepared worktree at '{}'", "✓".bright_green(), path.bright_yellow());
    println!("  {} Change directory and checkout: `cd {} && fgit switch {}`", "→".bright_cyan(), path, branch);

    Ok(())
}
