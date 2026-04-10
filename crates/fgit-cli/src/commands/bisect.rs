use std::env;
use std::fs;
use colored::Colorize;
use fgit_core::Repository;
use crate::commands::reset; // Relies on reset to forcefully checkout midpoints

pub fn start() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let branch = repo.refs().current_branch()?.unwrap_or_else(|| "HEAD".to_string());
    fs::write(repo.fgit_dir.join("BISECT_START"), branch).map_err(|e| e.to_string())?;

    // Clear old state just in case
    let _ = fs::remove_file(repo.fgit_dir.join("BISECT_BAD"));
    let _ = fs::remove_file(repo.fgit_dir.join("BISECT_GOOD"));

    println!("  {} Bisect session started.", "ℹ".bright_blue());
    Ok(())
}

pub fn bad(commit: Option<&str>) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    if !repo.fgit_dir.join("BISECT_START").exists() {
        return Err("You need to start by 'fgit bisect start'".to_string());
    }

    let target = match commit {
        Some(h) => h.to_string(),
        None => repo.refs().resolve_head()?.ok_or("No commits on HEAD")?,
    };

    fs::write(repo.fgit_dir.join("BISECT_BAD"), &target).map_err(|e| e.to_string())?;
    println!("  {} Commits starting from {} marked as bad.", "✗".bright_red(), &target[..8].bright_yellow());

    evaluate_bisect(&repo)
}

pub fn good(commit: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    if !repo.fgit_dir.join("BISECT_START").exists() {
        return Err("You need to start by 'fgit bisect start'".to_string());
    }

    let resolved = repo.refs().resolve_branch(commit)?.unwrap_or_else(|| commit.to_string());

    // We can append to GOOD list
    let good_file = repo.fgit_dir.join("BISECT_GOOD");
    let mut good_commits = if good_file.exists() {
        fs::read_to_string(&good_file).unwrap_or_default()
    } else {
        String::new()
    };
    good_commits.push_str(&resolved);
    good_commits.push('\n');
    fs::write(good_file, good_commits).map_err(|e| e.to_string())?;

    println!("  {} Commit {} marked as good.", "✓".bright_green(), &resolved[..8].bright_yellow());

    evaluate_bisect(&repo)
}

pub fn reset() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let start_file = repo.fgit_dir.join("BISECT_START");
    if !start_file.exists() {
        return Err("No bisect in progress".to_string());
    }

    let original_ref = fs::read_to_string(&start_file).unwrap_or_else(|_| "main".to_string());
    let _ = fs::remove_file(repo.fgit_dir.join("BISECT_START"));
    let _ = fs::remove_file(repo.fgit_dir.join("BISECT_BAD"));
    let _ = fs::remove_file(repo.fgit_dir.join("BISECT_GOOD"));

    println!("  {} Bisect complete. Checking out to original state '{}'.", "ℹ".bright_blue(), original_ref.bright_cyan());
    
    // Call strict reset.rs
    reset::run(&original_ref, "hard")
}

fn evaluate_bisect(repo: &Repository) -> Result<(), String> {
    let bad_file = repo.fgit_dir.join("BISECT_BAD");
    let good_file = repo.fgit_dir.join("BISECT_GOOD");

    if !bad_file.exists() || !good_file.exists() {
        return Ok(()); // waiting for more info
    }

    let bad_hash = fs::read_to_string(&bad_file).unwrap().trim().to_string();
    let goods: Vec<String> = fs::read_to_string(&good_file).unwrap().lines().map(|s| s.to_string()).collect();

    // Trace from BAD down, hoping to hit one of the GOOD.
    let mut chain = Vec::new();
    let mut curr = bad_hash.clone();
    let mut found_good = false;

    while let Ok(fgit_core::object::FgitObject::Commit(c)) = repo.read_object(&curr) {
        if goods.contains(&curr) {
            found_good = true;
            break;
        }
        chain.push(curr.clone());
        if c.parent_hashes.is_empty() { break; }
        curr = c.parent_hashes[0].clone();
    }

    if !found_good {
        return Err("The bad commit is not a descendent of the good commit(s).".to_string());
    }

    if chain.len() <= 1 {
        // chain only has the bad_hash
        println!("  {} Bisect finished! {} is the first bad commit.", "🎉".bright_magenta(), &bad_hash[..8].bright_yellow());
        let _ = fs::remove_file(repo.fgit_dir.join("BISECT_BAD"));
        let _ = fs::remove_file(repo.fgit_dir.join("BISECT_GOOD"));
        return Ok(());
    }

    // Binary search -> check out midpoint
    let mid_idx = chain.len() / 2;
    let mid_hash = &chain[mid_idx];

    let remaining = chain.len() - 1;
    println!("  {} Bisecting: {} revisions left to test after this (roughly {} steps)", 
        "ℹ".bright_blue(), remaining, (remaining as f64).log2().ceil() as i32);

    println!("  {} Checking out midpoint: {}", "→".bright_cyan(), &mid_hash[..8].bright_yellow());
    reset::run(mid_hash, "hard")?;

    println!("  {} Test your code, then run `fgit bisect good` or `fgit bisect bad`.", "⚠".bright_yellow());
    Ok(())
}
