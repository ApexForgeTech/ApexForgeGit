use fgit_core::{Repository, FgitObject};
use colored::Colorize;
use std::env;

pub fn run() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let refs = repo.refs();

    let current_hash = refs.resolve_head()?
        .ok_or("Nothing to undo — no commits yet")?;

    let commit = repo.read_object(&current_hash).map_err(|e| e.to_string())?;

    if let FgitObject::Commit(c) = commit {
        if c.parent_hashes.is_empty() {
            return Err("Cannot undo the initial commit".to_string());
        }

        let parent_hash = &c.parent_hashes[0];
        let branch = refs.current_branch()?
            .ok_or("Cannot undo in detached HEAD state")?;

        refs.update_branch(&branch, parent_hash)?;
        refs.append_reflog(&current_hash, parent_hash,
            &format!("undo: reverting commit {}", &current_hash[..8]))?;

        println!("  {} Undid commit {}", "✓".bright_green(), &current_hash[..8].bright_yellow());
        println!("  {} Now at: {}", "→".bright_yellow(), &parent_hash[..8].bright_yellow());
        println!("  {} \"{}\"", "💬", c.message.dimmed());
    } else {
        return Err("HEAD does not point to a commit".to_string());
    }

    Ok(())
}
