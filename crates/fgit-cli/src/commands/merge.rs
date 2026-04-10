use fgit_core::{Repository, FgitObject, Commit};
use fgit_core::object::Identity;
use colored::Colorize;
use std::env;

pub fn run(branch_name: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let refs = repo.refs();
    let config = repo.config().map_err(|e| e.to_string())?;

    let current_branch = refs.current_branch()?
        .ok_or("Cannot merge in detached HEAD state")?;

    let our_hash = refs.resolve_head()?
        .ok_or("Current branch has no commits")?;
    let their_hash = refs.resolve_branch(branch_name)?
        .ok_or_else(|| format!("Branch '{}' does not exist", branch_name))?;

    if our_hash == their_hash {
        println!("  {} {}", "ℹ".bright_blue(), "Already up to date.".dimmed());
        return Ok(());
    }

    // For now, create a merge commit pointing to both parents
    let our_commit = repo.read_object(&our_hash).map_err(|e| e.to_string())?;
    let _their_commit = repo.read_object(&their_hash).map_err(|e| e.to_string())?;

    let tree_hash = match &our_commit {
        FgitObject::Commit(c) => c.tree_hash.clone(),
        _ => return Err("Invalid commit object".to_string()),
    };

    let author_name = if config.user.name.is_empty() { "Unknown".to_string() } else { config.user.name.clone() };
    let author_email = if config.user.email.is_empty() { "unknown@apexforge.dev".to_string() } else { config.user.email.clone() };

    let merge_commit = Commit::new(
        tree_hash,
        vec![our_hash.clone(), their_hash.clone()],
        Identity::new(author_name.clone(), author_email.clone()),
        Identity::new(author_name, author_email),
        format!("Merge branch '{}' into {}", branch_name, current_branch),
    );

    let merge_obj = FgitObject::Commit(merge_commit);
    let merge_hash = repo.store_object(&merge_obj).map_err(|e| e.to_string())?;

    refs.update_branch(&current_branch, &merge_hash)?;
    refs.append_reflog(&our_hash, &merge_hash,
        &format!("merge {}: Merge branch '{}'", branch_name, branch_name))?;

    println!("  {} Merged '{}' into '{}'",
        "✓".bright_green(),
        branch_name.bright_cyan(),
        current_branch.bright_cyan()
    );
    println!("  {} Merge commit: {}",
        "→".bright_yellow(),
        &merge_hash[..8].bright_yellow()
    );

    Ok(())
}
