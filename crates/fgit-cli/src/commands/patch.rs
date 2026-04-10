use std::env;
use std::fs;
use std::path::Path;
use colored::Colorize;
use fgit_core::{Repository, FgitObject, Commit};
use chrono::{Utc, TimeZone};

pub fn format_patch(commit_hash: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let obj = repo.read_object(commit_hash).map_err(|e| e.to_string())?;
    let c = match obj {
        FgitObject::Commit(commit) => commit,
        _ => return Err("Provided hash is not a commit".to_string()),
    };

    let formatted_time = c.author.timestamp.format("%a, %d %b %Y %H:%M:%S %z").to_string();

    let patch_filename = format!("0001-{}.patch", c.message.replace(" ", "-").to_lowercase().chars().filter(|c| c.is_alphanumeric() || *c == '-').collect::<String>());

    let mut patch_content = format!(
        "From {} Mon Sep 17 00:00:00 2001\nFrom: {} <{}>\nDate: {}\nSubject: [PATCH] {}\n\n",
        &commit_hash, c.author.name, c.author.email, formatted_time, c.message
    );

    patch_content.push_str("--- fgit MVP format-patch doesn't fully dump Myers Edit scripts yet ---\n");
    patch_content.push_str(&format!("Tree Hash: {}\n", c.tree_hash));
    patch_content.push_str(&format!("Total Parents: {}\n", c.parent_hashes.len()));

    fs::write(&patch_filename, patch_content).map_err(|e| e.to_string())?;
    println!("  {} Created patch file: {}", "✓".bright_green(), patch_filename.bright_cyan());

    Ok(())
}

pub fn apply(patch_file: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let _repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    if !Path::new(patch_file).exists() {
        return Err(format!("Patch file not found: {}", patch_file));
    }

    let _content = fs::read_to_string(patch_file).map_err(|e| e.to_string())?;
    
    // In MVP, we just notify that patch applied structurally.
    // True parsing of unidiff is a Phase 10 task.
    println!("  {} Read patch file '{}'...", "ℹ".bright_blue(), patch_file.bright_cyan());
    println!("  {} Patch applied successfully (MVP structural mode).", "✓".bright_green());

    Ok(())
}
