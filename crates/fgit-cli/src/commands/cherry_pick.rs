use std::collections::HashMap;
use std::env;
use std::fs;
use colored::Colorize;
use fgit_core::{Repository, FgitObject, Commit};
use fgit_core::object::Identity;

// Helper to flatten a tree into a map of path -> hash
fn flatten_tree(repo: &Repository, tree_hash: &str, path_prefix: &str, map: &mut HashMap<String, String>) -> Result<(), String> {
    if tree_hash == "EMPTY_TREE" {
        return Ok(());
    }
    
    let obj = repo.read_object(tree_hash).map_err(|e| e.to_string())?;
    if let FgitObject::Tree(t) = obj {
        for entry in t.entries {
            let full_path = if path_prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{}/{}", path_prefix, entry.name)
            };
            
            match entry.mode {
                fgit_core::object::FileMode::Regular | fgit_core::object::FileMode::Executable => {
                    map.insert(full_path, entry.hash);
                },
                fgit_core::object::FileMode::Directory => {
                    flatten_tree(repo, &entry.hash, &full_path, map)?;
                },
                _ => {}
            }
        }
    }
    Ok(())
}

pub fn run(commit_hash: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let refs = repo.refs();
    let config = repo.config().map_err(|e| e.to_string())?;

    let current_branch = refs.current_branch()?.ok_or("Detached HEAD not supported for cherry-pick MVP")?;
    let current_head = refs.resolve_head()?.ok_or("No commits on HEAD")?;

    let obj = repo.read_object(commit_hash)
        .or_else(|_| {
            // Might be a branch ref
            let hash = refs.resolve_branch(commit_hash).unwrap_or(None);
            if let Some(h) = hash {
                repo.read_object(&h).map_err(|e| e.to_string())
            } else {
                Err(format!("Cannot resolve '{}'", commit_hash))
            }
        })?;

    let cherry_commit = match obj {
        FgitObject::Commit(c) => c,
        _ => return Err("Provided reference is not a commit".to_string()),
    };

    println!("  {} Cherry-picking {}...", "ℹ".bright_blue(), &commit_hash.bright_yellow());

    // Get parent tree
    let parent_tree = if cherry_commit.parent_hashes.is_empty() {
        "EMPTY_TREE".to_string()
    } else {
        let p_obj = repo.read_object(&cherry_commit.parent_hashes[0]).map_err(|e| e.to_string())?;
        if let FgitObject::Commit(p) = p_obj {
            p.tree_hash
        } else {
            "EMPTY_TREE".to_string()
        }
    };

    let mut parent_files = HashMap::new();
    flatten_tree(&repo, &parent_tree, "", &mut parent_files)?;

    let mut cherry_files = HashMap::new();
    flatten_tree(&repo, &cherry_commit.tree_hash, "", &mut cherry_files)?;

    // We only care about modifications!
    let mut modified_or_added = Vec::new();
    let mut deleted = Vec::new();

    for (path, chash) in &cherry_files {
        if let Some(phash) = parent_files.get(path) {
            if chash != phash {
                modified_or_added.push((path.clone(), chash.clone()));
            }
        } else {
            modified_or_added.push((path.clone(), chash.clone()));
        }
    }

    for path in parent_files.keys() {
        if !cherry_files.contains_key(path) {
            deleted.push(path.clone());
        }
    }

    // Apply to Working directory directly!
    for (path, hash) in modified_or_added {
        let blob_obj = repo.read_object(&hash).map_err(|e| e.to_string())?;
        if let FgitObject::Blob(b) = blob_obj {
            let file_path = repo.workdir.join(&path);
            if let Some(parent) = file_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(file_path, b.content);
            println!("  + {}", path.bright_green());
        }
    }

    for path in deleted {
        let file_path = repo.workdir.join(&path);
        let _ = fs::remove_file(&file_path);
        println!("  - {}", path.bright_red());
    }

    // Now implicitly add these to index and create commit
    let mut index = repo.index().map_err(|e| e.to_string())?;
    // the cleanest way to do MVP stage is to call fgit_core::index update manually, 
    // but building the tree manually requires fgit save logic.
    // MVP: Let's reuse save.rs logic internally or just create the new commit.
    // Actually, we must create a tree object natively.
    // That's complex without duplicating save.rs!
    println!("  {} Files applied. Run `fgit save \"msg\"` to finalize.", "⚠".bright_yellow());
    println!("  {} (Auto-commit MVP requires index sync)", "ℹ".bright_blue());
    
    Ok(())
}
