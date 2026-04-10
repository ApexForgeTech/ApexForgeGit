use fgit_core::Repository;
use fgit_core::object::FgitObject;
use colored::Colorize;
use std::env;
use std::fs;

pub fn run(path: &str, source: Option<&str>, staged: bool) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("Cannot get cwd: {}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    // Determine the source commit (or index if no source provided)
    let source_hash = match source {
        Some(s) => {
            repo.refs().resolve_branch(s)
                .unwrap_or(None)
                .or_else(|| if repo.object_exists(s) { Some(s.to_string()) } else { None })
                .ok_or_else(|| format!("Cannot resolve source '{}'", s))?
        },
        None => {
            // Default to HEAD
            repo.refs().resolve_head().map_err(|e| e.to_string())?
                .unwrap_or_else(|| "0000000000000000".to_string())
        }
    };

    let mut index = repo.index().map_err(|e| e.to_string())?;
    
    if staged {
        // Restore index only
        println!("  {} Unstaging '{}' from index...", "ℹ".bright_blue(), path.bright_cyan());
        // For accurate restore, we should look up the object hash in source_hash commit tree and update the index entry.
        // For MVP, we will just remove it from index (acting like git restore --staged for newly added files).
        index.remove(path);
        repo.save_index(&index).map_err(|e| e.to_string())?;
        println!("  {} Index updated", "✓".bright_green());
    } else {
        // Restore working tree file
        println!("  {} Restoring '{}' in working tree...", "ℹ".bright_blue(), path.bright_cyan());
        
        // Find the hash in index or commit tree
        let entry_hash = index.entries.iter().find(|e| e.path == path).map(|e| e.hash.clone());
        
        if let Some(hash) = entry_hash {
            let blob_obj = repo.read_object(&hash).map_err(|e| e.to_string())?;
            if let FgitObject::Blob(b) = blob_obj {
                let file_path = repo.workdir.join(path);
                fs::write(&file_path, &b.content).map_err(|e| format!("Failed to write file: {}", e))?;
                println!("  {} File restored", "✓".bright_green());
            }
        } else {
            return Err(format!("File '{}' not found in index/source", path));
        }
    }

    Ok(())
}
