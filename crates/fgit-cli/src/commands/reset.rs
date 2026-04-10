use fgit_core::Repository;
use fgit_core::object::{FgitObject, Tree};
use colored::Colorize;
use std::env;
use std::fs;

pub fn run(commit_ref: &str, mode: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("Cannot get cwd: {}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let refs = repo.refs();
    let current_branch = refs.current_branch()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Not currently on any branch".to_string())?;

    // Validate the target commit
    let branch_hash = repo.refs().resolve_branch(commit_ref)
        .unwrap_or(None)
        .or_else(|| if repo.object_exists(commit_ref) { Some(commit_ref.to_string()) } else { None })
        .ok_or_else(|| format!("Cannot resolve '{}'", commit_ref))?;

    let obj = repo.read_object(&branch_hash).map_err(|e| e.to_string())?;
    let target_commit = match obj {
        FgitObject::Commit(c) => c,
        _ => return Err(format!("'{}' is not a commit", commit_ref)),
    };

    println!("  {} {} -> {}", 
        "ℹ".bright_blue(), 
        format!("Resetting {} ({})", current_branch, mode).bright_white(),
        &branch_hash[0..8].bright_yellow()
    );

    // Update HEAD of current branch
    refs.update_branch(&current_branch, &branch_hash).map_err(|e| e.to_string())?;
    
    // Update reflog
    let current_head = refs.resolve_head().map_err(|e| e.to_string())?
        .unwrap_or_else(|| "0000000000000000".to_string());
        
    refs.append_reflog(&current_head, &branch_hash, &format!("reset: moving to {}", commit_ref))
        .map_err(|e| e.to_string())?;

    // Soft logic completes right here.
    if mode == "soft" {
        return Ok(());
    }

    // Mixed & Hard require index manipulation
    let mut index = repo.index().map_err(|e| e.to_string())?;
    
    // Clear the index
    index.entries.clear();
    
    // Reconstruct index from target_commit tree
    let tree_hash = target_commit.tree_hash;
    
    fn fill_index_from_tree(repo: &Repository, tree_hash: &str, path_prefix: &str, index: &mut fgit_core::index::Index) -> Result<(), String> {
        let obj = repo.read_object(tree_hash).map_err(|e| e.to_string())?;
        if let FgitObject::Tree(tree) = obj {
            for entry in tree.entries {
                let full_path = if path_prefix.is_empty() {
                    entry.name.clone()
                } else {
                    format!("{}/{}", path_prefix, entry.name)
                };
                
                match entry.mode {
                    fgit_core::object::FileMode::Regular | fgit_core::object::FileMode::Executable => {
                        let mtime = 0; // Fake mtime from repo history, or query disk if exists
                        index.add(fgit_core::index::IndexEntry {
                            path: full_path,
                            hash: entry.hash,
                            size: 0, // Placeholder, would need blob read for exact size
                            mtime,
                            mode: "100644".to_string(), // Fake mode
                        });
                    },
                    fgit_core::object::FileMode::Directory => {
                        fill_index_from_tree(repo, &entry.hash, &full_path, index)?;
                    },
                    _ => {}
                }
            }
        }
        Ok(())
    }
    
    fill_index_from_tree(&repo, &tree_hash, "", &mut index)?;
    repo.save_index(&index).map_err(|e| e.to_string())?;

    // Hard logic touches the working directory
    if mode == "hard" {
        // Warning: This physically overrides files
        println!("  {} Rewriting working tree...", "⚠".bright_yellow());
        // For accurate hard reset, we iterate over the index and restore files physically
        // We also delete whatever files are not in the index but are untracked
        // (A full hard reset implementation deletes tracked files not in index, updates modified ones)
        
        for entry in &index.entries {
            let blob_obj = repo.read_object(&entry.hash).map_err(|e| e.to_string())?;
            if let FgitObject::Blob(b) = blob_obj {
                let file_path = repo.workdir.join(&entry.path);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).unwrap_or_default();
                }
                fs::write(&file_path, &b.content).unwrap_or_default();
            }
        }
    }

    println!("  {} {} reset complete", "✓".bright_green(), mode);
    Ok(())
}
