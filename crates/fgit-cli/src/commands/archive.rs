use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use colored::Colorize;
use fgit_core::{Repository, FgitObject, Commit};
use std::process::Command;

fn export_tree(repo: &Repository, tree_hash: &str, output_dir: &Path) -> Result<(), String> {
    if tree_hash == "EMPTY_TREE" {
        return Ok(());
    }

    let obj = repo.read_object(tree_hash).map_err(|e| e.to_string())?;
    if let FgitObject::Tree(t) = obj {
        for entry in t.entries {
            let dest_path = output_dir.join(&entry.name);
            
            match entry.mode {
                fgit_core::object::FileMode::Regular | fgit_core::object::FileMode::Executable => {
                    let blob_obj = repo.read_object(&entry.hash).map_err(|e| e.to_string())?;
                    if let FgitObject::Blob(b) = blob_obj {
                        if let Some(parent) = dest_path.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        fs::write(&dest_path, b.content).map_err(|e| e.to_string())?;
                    }
                },
                fgit_core::object::FileMode::Directory => {
                    export_tree(repo, &entry.hash, &dest_path)?;
                },
                _ => {}
            }
        }
    }

    Ok(())
}

pub fn run(commit_hash: Option<&str>, output_file: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let target_hash = match commit_hash {
        Some(h) => h.to_string(),
        None => repo.refs().resolve_head()?.ok_or("No commits on HEAD")?,
    };

    let obj = repo.read_object(&target_hash).map_err(|e| e.to_string())?;
    let tree_hash = match obj {
        FgitObject::Commit(c) => c.tree_hash,
        _ => return Err("The specified reference is not a commit".to_string()),
    };

    // Create a temporary directory
    let temp_dir = env::temp_dir().join(format!("fgit_archive_{}", target_hash));
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    println!("  {} Constructing tree for {}...", "ℹ".bright_blue(), &target_hash[..8].bright_yellow());
    export_tree(&repo, &tree_hash, &temp_dir)?;

    println!("  {} Compressing into '{}'...", "ℹ".bright_blue(), output_file.bright_cyan());
    
    // Determine output format based on extension
    let is_tar = output_file.ends_with(".tar.gz");
    
    let status = if is_tar {
        Command::new("tar")
            .arg("-czf")
            .arg(output_file)
            .arg("-C")
            .arg(&temp_dir)
            .arg(".")
            .status()
    } else {
        // Zip by default
        Command::new("zip")
            .arg("-r")
            .arg(output_file)
            .arg(".")
            .current_dir(&temp_dir)
            .status()
    };

    let _ = fs::remove_dir_all(&temp_dir);

    match status {
        Ok(s) if s.success() => {
            println!("  {} Exported commit cleanly to archive.", "✓".bright_green());
            Ok(())
        },
        _ => Err("Failed to create archive using system tools (zip/tar required).".to_string())
    }
}
