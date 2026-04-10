use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// Manages branches, HEAD, and ref pointers
#[derive(Debug)]
pub struct RefManager {
    fgit_dir: PathBuf,
}

/// Branch metadata — Git doesn't store this, ApexForge Git does
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub commit_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub description: String,
}

impl RefManager {
    pub fn new(fgit_dir: PathBuf) -> Self {
        Self { fgit_dir }
    }

    /// Read HEAD — returns either "ref: refs/heads/<branch>" or a commit hash
    pub fn read_head(&self) -> Result<String, String> {
        fs::read_to_string(self.fgit_dir.join("HEAD"))
            .map(|s| s.trim().to_string())
            .map_err(|e| format!("Failed to read HEAD: {}", e))
    }

    /// Get the current branch name (None if detached HEAD)
    pub fn current_branch(&self) -> Result<Option<String>, String> {
        let head = self.read_head()?;
        if let Some(ref_path) = head.strip_prefix("ref: refs/heads/") {
            Ok(Some(ref_path.to_string()))
        } else {
            Ok(None) // detached HEAD
        }
    }

    /// Resolve HEAD to a commit hash
    pub fn resolve_head(&self) -> Result<Option<String>, String> {
        let head = self.read_head()?;
        if let Some(ref_path) = head.strip_prefix("ref: ") {
            let ref_file = self.fgit_dir.join(ref_path);
            if ref_file.exists() {
                let hash = fs::read_to_string(&ref_file)
                    .map_err(|e| format!("Failed to read ref: {}", e))?;
                Ok(Some(hash.trim().to_string()))
            } else {
                Ok(None) // branch exists but has no commits
            }
        } else {
            Ok(Some(head)) // detached HEAD, already a hash
        }
    }

    /// Set HEAD to point to a branch
    pub fn set_head_to_branch(&self, branch: &str) -> Result<(), String> {
        fs::write(
            self.fgit_dir.join("HEAD"),
            format!("ref: refs/heads/{}\n", branch),
        ).map_err(|e| format!("Failed to write HEAD: {}", e))
    }

    /// Set HEAD to a specific commit (detached)
    pub fn set_head_detached(&self, hash: &str) -> Result<(), String> {
        fs::write(self.fgit_dir.join("HEAD"), format!("{}\n", hash))
            .map_err(|e| format!("Failed to write HEAD: {}", e))
    }

    /// Update a branch ref to point to a commit
    pub fn update_branch(&self, branch: &str, hash: &str) -> Result<(), String> {
        let ref_path = self.fgit_dir.join("refs/heads").join(branch);
        fs::write(&ref_path, format!("{}\n", hash))
            .map_err(|e| format!("Failed to update branch {}: {}", branch, e))
    }

    /// Get the commit hash a branch points to
    pub fn resolve_branch(&self, branch: &str) -> Result<Option<String>, String> {
        let ref_path = self.fgit_dir.join("refs/heads").join(branch);
        if ref_path.exists() {
            let hash = fs::read_to_string(&ref_path)
                .map_err(|e| format!("Failed to read branch {}: {}", branch, e))?;
            Ok(Some(hash.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    /// Create a new branch pointing to a commit
    pub fn create_branch(&self, name: &str, hash: &str) -> Result<(), String> {
        let ref_path = self.fgit_dir.join("refs/heads").join(name);
        if ref_path.exists() {
            return Err(format!("Branch '{}' already exists", name));
        }
        fs::write(&ref_path, format!("{}\n", hash))
            .map_err(|e| format!("Failed to create branch {}: {}", name, e))
    }

    /// Delete a branch
    pub fn delete_branch(&self, name: &str) -> Result<(), String> {
        // Prevent deleting current branch
        if let Ok(Some(current)) = self.current_branch() {
            if current == name {
                return Err(format!("Cannot delete current branch '{}'", name));
            }
        }
        let ref_path = self.fgit_dir.join("refs/heads").join(name);
        if !ref_path.exists() {
            return Err(format!("Branch '{}' not found", name));
        }
        fs::remove_file(&ref_path)
            .map_err(|e| format!("Failed to delete branch {}: {}", name, e))
    }

    /// List all branches
    pub fn list_branches(&self) -> Result<Vec<String>, String> {
        let heads_dir = self.fgit_dir.join("refs/heads");
        if !heads_dir.exists() {
            return Ok(Vec::new());
        }
        let mut branches = Vec::new();
        let entries = fs::read_dir(&heads_dir)
            .map_err(|e| format!("Failed to list branches: {}", e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
            if let Some(name) = entry.file_name().to_str() {
                branches.push(name.to_string());
            }
        }
        branches.sort();
        Ok(branches)
    }

    /// Append to reflog
    pub fn append_reflog(&self, old: &str, new: &str, msg: &str) -> Result<(), String> {
        let log_path = self.fgit_dir.join("logs/HEAD");
        let entry = format!(
            "{} {} {} {}\n",
            old, new,
            chrono::Utc::now().to_rfc3339(),
            msg
        );
        let mut content = fs::read_to_string(&log_path).unwrap_or_default();
        content.push_str(&entry);
        fs::write(&log_path, content)
            .map_err(|e| format!("Failed to write reflog: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, RefManager) {
        let tmp = TempDir::new().unwrap();
        let fgit = tmp.path().join(".fgit");
        fs::create_dir_all(fgit.join("refs/heads")).unwrap();
        fs::create_dir_all(fgit.join("logs")).unwrap();
        fs::write(fgit.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(fgit.join("logs/HEAD"), "").unwrap();
        (tmp, RefManager::new(fgit))
    }

    #[test]
    fn test_current_branch() {
        let (_tmp, refs) = setup();
        assert_eq!(refs.current_branch().unwrap(), Some("main".to_string()));
    }

    #[test]
    fn test_create_and_resolve_branch() {
        let (_tmp, refs) = setup();
        refs.create_branch("feature", "abc123").unwrap();
        assert_eq!(refs.resolve_branch("feature").unwrap(), Some("abc123".to_string()));
    }

    #[test]
    fn test_list_branches() {
        let (_tmp, refs) = setup();
        refs.create_branch("alpha", "h1").unwrap();
        refs.create_branch("beta", "h2").unwrap();
        let list = refs.list_branches().unwrap();
        assert_eq!(list, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_delete_branch() {
        let (_tmp, refs) = setup();
        refs.create_branch("temp", "h1").unwrap();
        refs.delete_branch("temp").unwrap();
        assert!(refs.resolve_branch("temp").unwrap().is_none());
    }

    #[test]
    fn test_cannot_delete_current_branch() {
        let (_tmp, refs) = setup();
        refs.update_branch("main", "h1").unwrap();
        assert!(refs.delete_branch("main").is_err());
    }
}
