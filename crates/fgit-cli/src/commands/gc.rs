use fgit_core::{Repository, FgitObject, ObjectType, Tree};
use colored::Colorize;
use std::collections::HashSet;
use std::env;
use std::fs;
use walkdir::WalkDir;

pub fn run() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("Cannot get cwd: {}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    println!("  {} Starting garbage collection...", "ℹ".bright_blue());

    let mut reachable: HashSet<String> = HashSet::new();
    let mut to_visit: Vec<String> = Vec::new();

    let refs = repo.refs();
    // Start with all branches
    if let Ok(branches) = refs.list_branches() {
        for branch in branches {
            if let Ok(Some(hash)) = refs.resolve_branch(&branch) {
                to_visit.push(hash);
            }
        }
    }
    
    // Add current HEAD explicitly just in case
    if let Ok(Some(hash)) = refs.resolve_head() {
        to_visit.push(hash);
    }

    // Traverse the graph to mark reachable objects
    while let Some(hash) = to_visit.pop() {
        if reachable.contains(&hash) {
            continue;
        }
        
        reachable.insert(hash.clone());
        
        let obj = match repo.read_object(&hash) {
            Ok(o) => o,
            Err(_) => continue, // Corrupt or missing object, ignore for now
        };

        match obj {
            FgitObject::Commit(commit) => {
                to_visit.push(commit.tree_hash.clone());
                for parent in commit.parent_hashes {
                    to_visit.push(parent);
                }
            },
            FgitObject::Tree(tree) => {
                for entry in tree.entries {
                    to_visit.push(entry.hash);
                }
            },
            _ => {
                // Blob or Tag have no further references in this MVP
            }
        }
    }

    println!("  {} Marked {} objects as reachable.", "✓".bright_green(), reachable.len());
    
    // Sweep phase: delete unmarked objects
    let objects_dir = repo.fgit_dir.join("objects");
    let mut deleted_count = 0;
    let mut freed_bytes = 0;

    for entry in WalkDir::new(&objects_dir)
        .min_depth(2) // objects/xx/yyyy...
        .max_depth(2)
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, 
        };
        
        if !entry.file_type().is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy();
        let parent_name = entry.path().parent().unwrap().file_name().unwrap().to_string_lossy();
        
        let full_hash = format!("{}{}", parent_name, file_name);

        if !reachable.contains(&full_hash) {
            if let Ok(metadata) = entry.metadata() {
                freed_bytes += metadata.len();
            }
            if fs::remove_file(entry.path()).is_ok() {
                deleted_count += 1;
            }
        }
    }

    if deleted_count > 0 {
        println!("  {} Deleted {} unreachable loose objects ({} bytes freed)", 
            "🗑".bright_yellow(), 
            deleted_count.to_string().bright_cyan(),
            format_size(freed_bytes).bright_magenta()
        );
    } else {
        println!("  {} No garbage found.", "✨".bright_green());
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
