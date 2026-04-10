use fgit_core::{Repository, FgitObject, Blob, Commit};
use fgit_core::object::Identity;
use fgit_core::index::IndexEntry;
use colored::Colorize;
use std::env;
use std::fs;
use std::io::{self, Write};
use walkdir::WalkDir;

pub fn run(message: &str, interactive: bool) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("Cannot get cwd: {}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    if message.is_empty() {
        return Err("Commit message is required. Usage: fgit save \"your message\"".to_string());
    }

    // Run pre-save hook
    repo.hooks().run("pre-save")?;

    let config = repo.config().map_err(|e| e.to_string())?;
    let ignore_rules = repo.ignore_rules();
    let mut index = repo.index().map_err(|e| e.to_string())?;

    // Walk working directory and stage all non-ignored files
    let mut file_count = 0u32;
    let mut total_size = 0u64;

    for entry in WalkDir::new(&repo.workdir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            name != ".fgit" && name != ".git"
        })
    {
        let entry = entry.map_err(|e| format!("Walk error: {}", e))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let rel_path = entry.path().strip_prefix(&repo.workdir)
            .map_err(|e| format!("Path error: {}", e))?;
        let rel_str = rel_path.to_str().ok_or("Invalid path encoding")?;

        // Check ignore rules
        let metadata = entry.metadata().map_err(|e| format!("Metadata error: {}", e))?;
        if ignore_rules.is_ignored(rel_str, Some(metadata.len())) {
            continue;
        }

        if interactive {
            print!("  {} Stage file '{}'? [y/N] ", "?".bright_yellow(), rel_str.bright_cyan());
            io::stdout().flush().unwrap_or_default();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap_or_default();
            if !input.trim().eq_ignore_ascii_case("y") {
                continue;
            }
        }

        // Hash the file and store as blob
        let content = fs::read(entry.path()).map_err(|e| format!("Read error: {}", e))?;
        let blob = FgitObject::Blob(Blob::new(content));
        let hash = repo.store_object(&blob).map_err(|e| e.to_string())?;

        let mtime = metadata.modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);

        index.add(IndexEntry {
            path: rel_str.to_string(),
            hash: hash.clone(),
            size: metadata.len(),
            mtime,
            mode: "100644".to_string(),
        });

        file_count += 1;
        total_size += metadata.len();
    }

    if file_count == 0 {
        println!("{}", "⚠  Nothing to save — working tree is clean.".bright_yellow());
        return Ok(());
    }

    // Build tree from index
    let tree_hash = repo.build_tree_from_index(&index).map_err(|e| e.to_string())?;

    // Get parent commit
    let refs = repo.refs();
    let parent_hash = refs.resolve_head().map_err(|e| e.to_string())?;
    let parent_hashes = parent_hash.iter().cloned().collect::<Vec<_>>();

    // Create commit
    let author_name = if config.user.name.is_empty() { "Unknown".to_string() } else { config.user.name.clone() };
    let author_email = if config.user.email.is_empty() { "unknown@apexforge.dev".to_string() } else { config.user.email.clone() };
    let author = Identity::new(author_name.clone(), author_email.clone());
    let committer = Identity::new(author_name, author_email);

    let commit = Commit::new(tree_hash, parent_hashes.clone(), author, committer, message.to_string());
    let commit_obj = FgitObject::Commit(commit);
    let commit_hash = repo.store_object(&commit_obj).map_err(|e| e.to_string())?;

    // Update branch ref
    if let Some(branch) = refs.current_branch().map_err(|e| e.to_string())? {
        refs.update_branch(&branch, &commit_hash).map_err(|e| e.to_string())?;
    }

    // Save index
    repo.save_index(&index).map_err(|e| e.to_string())?;

    // Append reflog
    let old_hash = parent_hashes.first().map_or("0000000000000000", |h| h.as_str());
    refs.append_reflog(old_hash, &commit_hash, &format!("save: {}", message))
        .map_err(|e| e.to_string())?;

    // Output
    let short_hash = &commit_hash[..8];
    let branch_name = refs.current_branch().unwrap_or(Some("detached".to_string()))
        .unwrap_or_else(|| "detached".to_string());

    println!();
    println!("  {} {} {}",
        "✓".bright_green().bold(),
        format!("[{}]", branch_name).bright_cyan(),
        format!("{}", short_hash).bright_yellow()
    );
    println!("  {} {}", "💬".to_string(), message.bright_white());
    println!("  {} {} files saved ({} bytes)",
        "📦".to_string(),
        file_count.to_string().bright_green(),
        format_size(total_size).dimmed()
    );
    println!();

    // Run post-save hook (errors here do not fail the commit)
    if let Err(e) = repo.hooks().run("post-save") {
        println!("  {} {}", "⚠  Post-save hook error:".bright_yellow(), e);
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
