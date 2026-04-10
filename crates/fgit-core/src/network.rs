use std::path::{Path, PathBuf};
use std::fs;
use crate::repository::{Repository, RepoResult, RepoError};

/// Network Engine for synchronizing with remotes
/// First version builds a "Local Sync" mechanism for P2P over mounted/local filesystems.
pub struct NetworkEngine;

impl NetworkEngine {
    /// Clone a repository from a local path to a target directory
    pub fn clone_local(source_path: &Path, target_dir: &Path) -> RepoResult<Repository> {
        let source_fgit = source_path.join(".fgit");
        if !source_fgit.exists() {
            return Err(RepoError::General("Source is not an ApexForge Git repository".to_string()));
        }

        // Initialize target repository
        let repo = Repository::init(target_dir)?;

        // Copy objects
        Self::copy_dir_recursive(&source_fgit.join("objects"), &repo.fgit_dir.join("objects"))?;
        
        // Copy refs
        Self::copy_dir_recursive(&source_fgit.join("refs"), &repo.fgit_dir.join("refs"))?;

        // Checkout HEAD
        let refs = repo.refs();
        if let Ok(Some(head_hash)) = refs.resolve_head() {
            println!("Cloned cleanly, HEAD is at {}", &head_hash[..8]);
            // In a real implementation we would restore working tree here.
            // For MVP, we'll keep it simple.
        }

        Ok(repo)
    }

    /// Push local objects/refs to remote
    pub fn push(repo: &Repository, remote_path: &Path) -> RepoResult<()> {
        let remote_str = remote_path.to_string_lossy();
        if remote_str.starts_with("http://") || remote_str.starts_with("https://") {
            println!("Connecting to {} via HTTP...", remote_str);
            // Stubbed HTTP push using reqwest
            // let _client = reqwest::blocking::Client::new();
            println!("HTTP sync is simulated for MVP. Fast-forwarding push.");
            return Ok(());
        }

        let remote_fgit = remote_path.join(".fgit");
        if !remote_fgit.exists() {
            return Err(RepoError::General("Remote is not an fgit repository".to_string()));
        }

        // Sync objects (simple recursive copy without overwrite if exists)
        Self::copy_dir_sync(&repo.fgit_dir.join("objects"), &remote_fgit.join("objects"))?;
        
        // Push refs
        Self::copy_dir_sync(&repo.fgit_dir.join("refs/heads"), &remote_fgit.join("refs/heads"))?;
        Self::copy_dir_sync(&repo.fgit_dir.join("refs/tags"), &remote_fgit.join("refs/tags"))?;

        Ok(())
    }

    /// Pull remote objects/refs to local
    pub fn pull(repo: &Repository, remote_path: &Path) -> RepoResult<()> {
        let remote_fgit = remote_path.join(".fgit");
        if !remote_fgit.exists() {
            return Err(RepoError::General("Remote is not an fgit repository".to_string()));
        }

        // Fetch objects
        Self::copy_dir_sync(&remote_fgit.join("objects"), &repo.fgit_dir.join("objects"))?;

        // Pull refs
        Self::copy_dir_sync(&remote_fgit.join("refs/heads"), &repo.fgit_dir.join("refs/remotes/origin"))?;
        Self::copy_dir_sync(&remote_fgit.join("refs/tags"), &repo.fgit_dir.join("refs/tags"))?;

        Ok(())
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> RepoResult<()> {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());

            if path.is_dir() {
                Self::copy_dir_recursive(&path, &dest_path)?;
            } else {
                fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    fn copy_dir_sync(src: &Path, dst: &Path) -> RepoResult<()> {
        if !src.exists() { return Ok(()); }
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());

            if path.is_dir() {
                Self::copy_dir_sync(&path, &dest_path)?;
            } else if !dest_path.exists() {
                fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }
}
