use fgit_core::Repository;
use colored::Colorize;
use std::env;
use std::fs;
use walkdir::WalkDir;

pub fn run() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let index = repo.index().map_err(|e| e.to_string())?;
    let ignore = repo.ignore_rules();
    let refs = repo.refs();

    let branch = refs.current_branch().unwrap_or(None)
        .unwrap_or_else(|| "detached HEAD".to_string());

    println!();
    println!("  {} {}", "On branch".dimmed(), branch.bright_cyan().bold());
    println!();

    let mut untracked = Vec::new();
    let mut modified = Vec::new();

    for entry in WalkDir::new(&repo.workdir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            name != ".fgit" && name != ".git"
        })
    {
        let entry = match entry { Ok(e) => e, Err(_) => continue };
        if !entry.file_type().is_file() { continue; }

        let rel = match entry.path().strip_prefix(&repo.workdir) {
            Ok(r) => r.to_str().unwrap_or("").to_string(),
            Err(_) => continue,
        };

        let meta = entry.metadata().unwrap_or_else(|_| fs::metadata(entry.path()).unwrap());
        if ignore.is_ignored(&rel, Some(meta.len())) { continue; }

        match index.get(&rel) {
            None => untracked.push(rel),
            Some(idx_entry) => {
                let content = fs::read(entry.path()).unwrap_or_default();
                let blob = fgit_core::FgitObject::Blob(fgit_core::Blob::new(content));
                let hash = blob.hash();
                if hash != idx_entry.hash {
                    modified.push(rel);
                }
            }
        }
    }

    if modified.is_empty() && untracked.is_empty() {
        println!("  {} {}", "✓".bright_green(), "Working tree is clean".bright_white());
    } else {
        if !modified.is_empty() {
            println!("  {} ({} file{})", "Modified:".bright_yellow().bold(),
                modified.len(), if modified.len() > 1 { "s" } else { "" });
            for f in &modified {
                println!("    {} {}", "~".bright_yellow(), f);
            }
            println!();
        }
        if !untracked.is_empty() {
            println!("  {} ({} file{})", "Untracked:".bright_red().bold(),
                untracked.len(), if untracked.len() > 1 { "s" } else { "" });
            for f in &untracked {
                println!("    {} {}", "?".bright_red(), f);
            }
            println!();
        }
    }

    Ok(())
}
