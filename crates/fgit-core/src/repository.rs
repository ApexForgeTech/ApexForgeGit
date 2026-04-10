use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use crate::object::{FgitObject, Blob, Tree, FileMode};
use crate::config::FgitConfig;
use crate::ignore::IgnoreRules;
use crate::index::Index;
use crate::refs::RefManager;
use crate::hooks::HookManager;

/// The name of the ApexForge Git directory (replaces .git)
pub const FGIT_DIR: &str = ".fgit";

#[derive(Error, Debug)]
pub enum RepoError {
    #[error("Not an fgit repository (or any parent): {0}")]
    NotARepository(String),
    #[error("Repository already exists at {0}")]
    AlreadyExists(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    #[error("Corrupt object: {0}")]
    CorruptObject(String),
    #[error("Invalid reference: {0}")]
    InvalidRef(String),
    #[error("Merge conflict in: {0}")]
    MergeConflict(String),
    #[error("{0}")]
    General(String),
}

pub type RepoResult<T> = Result<T, RepoError>;

/// Main repository handle — the heart of ApexForge Git
#[derive(Debug)]
pub struct Repository {
    /// Root directory of the working tree
    pub workdir: PathBuf,
    /// Path to .fgit directory
    pub fgit_dir: PathBuf,
}

impl Repository {
    /// Initialize a new repository in the given directory
    pub fn init(path: &Path) -> RepoResult<Self> {
        let workdir = path.to_path_buf();
        let fgit_dir = workdir.join(FGIT_DIR);

        if fgit_dir.exists() {
            return Err(RepoError::AlreadyExists(workdir.display().to_string()));
        }

        // Create the .fgit directory structure
        fs::create_dir_all(&fgit_dir)?;
        fs::create_dir_all(fgit_dir.join("objects"))?;
        fs::create_dir_all(fgit_dir.join("objects/packs"))?;
        fs::create_dir_all(fgit_dir.join("refs/heads"))?;
        fs::create_dir_all(fgit_dir.join("refs/tags"))?;
        fs::create_dir_all(fgit_dir.join("refs/remotes"))?;
        fs::create_dir_all(fgit_dir.join("stash"))?;
        fs::create_dir_all(fgit_dir.join("hooks"))?;
        fs::create_dir_all(fgit_dir.join("logs"))?;

        // Write HEAD pointing to main branch
        fs::write(fgit_dir.join("HEAD"), "ref: refs/heads/main\n")?;

        // Write default description
        fs::write(
            fgit_dir.join("description"),
            "Unnamed ApexForge Git repository\n",
        )?;

        // Write default config.toml
        let default_config = FgitConfig::default();
        let config_str = toml::to_string_pretty(&default_config)
            .map_err(|e| RepoError::General(format!("Failed to serialize config: {}", e)))?;
        fs::write(fgit_dir.join("config.toml"), config_str)?;

        // Write empty index
        let empty_index = Index::new();
        let index_data = serde_json::to_vec_pretty(&empty_index)
            .map_err(|e| RepoError::General(format!("Failed to serialize index: {}", e)))?;
        fs::write(fgit_dir.join("index"), index_data)?;

        // Initialize empty reflog
        fs::write(fgit_dir.join("logs/HEAD"), "")?;

        // Create default .fgitignore in workdir if it doesn't exist
        let fgitignore_path = workdir.join(".fgitignore");
        if !fgitignore_path.exists() {
            let default_ignore = Self::generate_default_ignore(&workdir);
            fs::write(fgitignore_path, default_ignore)?;
        }

        Ok(Self { workdir, fgit_dir })
    }

    /// Open an existing repository by finding .fgit in current or parent dirs
    pub fn open(path: &Path) -> RepoResult<Self> {
        let mut curr = path.to_path_buf();
        loop {
            let fgit = curr.join(".fgit");
            if fgit.exists() {
                // Worktree checking
                let mut actual_fgit_dir = fgit.clone();
                if fgit.is_file() {
                    let content = std::fs::read_to_string(&fgit).map_err(|_| RepoError::General("Cannot read .fgit file".to_string()))?;
                    if content.starts_with("gitdir: ") {
                        let linked = content.replace("gitdir: ", "").trim().to_string();
                        actual_fgit_dir = std::path::PathBuf::from(linked);
                    }
                }
                
                return Ok(Repository {
                    fgit_dir: actual_fgit_dir,
                    workdir: curr,
                });
            }
            if !curr.pop() {
                break;
            }
        }
        Err(RepoError::NotARepository(path.display().to_string()))
    }

    /// Store an object in the object store
    pub fn store_object(&self, obj: &FgitObject) -> RepoResult<String> {
        let hash = obj.hash();
        let serialized = obj.serialize();

        // Use first 2 chars as subdirectory (like Git, but in objects/)
        let (dir_name, file_name) = hash.split_at(2);
        let obj_dir = self.fgit_dir.join("objects").join(dir_name);
        fs::create_dir_all(&obj_dir)?;

        let obj_path = obj_dir.join(file_name);
        if !obj_path.exists() {
            // Compress with zstd before writing
            let compressed = zstd::encode_all(serialized.as_slice(), 3)
                .map_err(|e| RepoError::General(format!("Compression failed: {}", e)))?;
            fs::write(&obj_path, compressed)?;
        }

        Ok(hash)
    }

    /// Read an object from the object store
    pub fn read_object(&self, hash: &str) -> RepoResult<FgitObject> {
        if hash.len() < 4 {
            return Err(RepoError::ObjectNotFound(hash.to_string()));
        }

        let (dir_name, file_name) = hash.split_at(2);
        let obj_path = self.fgit_dir.join("objects").join(dir_name).join(file_name);

        if !obj_path.exists() {
            return Err(RepoError::ObjectNotFound(hash.to_string()));
        }

        let compressed = fs::read(&obj_path)?;
        let data = zstd::decode_all(compressed.as_slice())
            .map_err(|e| RepoError::CorruptObject(format!("Decompression failed: {}", e)))?;

        FgitObject::deserialize(&data)
            .map_err(|e| RepoError::CorruptObject(e))
    }

    /// Check if an object exists in the store
    pub fn object_exists(&self, hash: &str) -> bool {
        if hash.len() < 4 {
            return false;
        }
        let (dir_name, file_name) = hash.split_at(2);
        self.fgit_dir.join("objects").join(dir_name).join(file_name).exists()
    }

    /// Hash a file from the working directory and store it as a blob
    pub fn hash_file(&self, file_path: &Path) -> RepoResult<String> {
        let abs_path = if file_path.is_relative() {
            self.workdir.join(file_path)
        } else {
            file_path.to_path_buf()
        };

        let content = fs::read(&abs_path)?;
        let blob = FgitObject::Blob(Blob::new(content));
        self.store_object(&blob)
    }

    /// Build a tree object from the current index
    pub fn build_tree_from_index(&self, index: &Index) -> RepoResult<String> {
        self.build_tree_recursive(index, "")
    }

    /// Recursively build tree objects from index entries
    fn build_tree_recursive(&self, index: &Index, prefix: &str) -> RepoResult<String> {
        let mut tree = Tree::new();
        let mut seen_dirs: std::collections::HashSet<String> = std::collections::HashSet::new();

        for entry in &index.entries {
            let rel_path = if prefix.is_empty() {
                entry.path.clone()
            } else if let Some(stripped) = entry.path.strip_prefix(&format!("{}/", prefix)) {
                stripped.to_string()
            } else {
                continue;
            };

            if let Some(slash_pos) = rel_path.find('/') {
                // This entry is in a subdirectory
                let dir_name = &rel_path[..slash_pos];
                if seen_dirs.insert(dir_name.to_string()) {
                    let sub_prefix = if prefix.is_empty() {
                        dir_name.to_string()
                    } else {
                        format!("{}/{}", prefix, dir_name)
                    };
                    let subtree_hash = self.build_tree_recursive(index, &sub_prefix)?;
                    tree.add_entry(FileMode::Directory, dir_name.to_string(), subtree_hash);
                }
            } else {
                // Direct file in this tree level
                tree.add_entry(
                    FileMode::Regular,
                    rel_path.to_string(),
                    entry.hash.clone(),
                );
            }
        }

        tree.sort_entries();
        let tree_obj = FgitObject::Tree(tree);
        self.store_object(&tree_obj)
    }

    /// Get the RefManager for this repository
    pub fn refs(&self) -> RefManager {
        RefManager::new(self.fgit_dir.clone())
    }

    /// Get the index for this repository
    pub fn index(&self) -> RepoResult<Index> {
        let index_path = self.fgit_dir.join("index");
        if index_path.exists() {
            let data = fs::read(&index_path)?;
            serde_json::from_slice(&data)
                .map_err(|e| RepoError::General(format!("Failed to read index: {}", e)))
        } else {
            Ok(Index::new())
        }
    }

    /// Save the index to disk
    pub fn save_index(&self, index: &Index) -> RepoResult<()> {
        let data = serde_json::to_vec_pretty(index)
            .map_err(|e| RepoError::General(format!("Failed to serialize index: {}", e)))?;
        fs::write(self.fgit_dir.join("index"), data)?;
        Ok(())
    }

    /// Get ignore rules for this repository
    pub fn ignore_rules(&self) -> IgnoreRules {
        let fgitignore_path = self.workdir.join(".fgitignore");
        if fgitignore_path.exists() {
            if let Ok(content) = fs::read_to_string(&fgitignore_path) {
                return IgnoreRules::parse(&content);
            }
        }
        IgnoreRules::empty()
    }

    /// Access the HookManager
    pub fn hooks(&self) -> HookManager {
        HookManager::new(&self.fgit_dir)
    }

    /// Get config for this repository
    pub fn config(&self) -> RepoResult<FgitConfig> {
        let config_path = self.fgit_dir.join("config.toml");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            toml::from_str(&content)
                .map_err(|e| RepoError::General(format!("Failed to parse config: {}", e)))
        } else {
            Ok(FgitConfig::default())
        }
    }

    /// Save config to disk
    pub fn save_config(&self, config: &FgitConfig) -> RepoResult<()> {
        let content = toml::to_string_pretty(config)
            .map_err(|e| RepoError::General(format!("Failed to serialize config: {}", e)))?;
        fs::write(self.fgit_dir.join("config.toml"), content)?;
        Ok(())
    }

    /// Generate default .fgitignore based on auto-detection of the project type
    fn generate_default_ignore(workdir: &Path) -> String {
        let mut lines = vec![
            "# ═══════════════════════════════════════════════════".to_string(),
            "# ApexForge Git — Auto-generated .fgitignore".to_string(),
            "# ═══════════════════════════════════════════════════".to_string(),
            "".to_string(),
            "# ApexForge Git internals".to_string(),
            ".fgit/".to_string(),
            "".to_string(),
            "# OS generated files".to_string(),
            "[os]".to_string(),
            ".DS_Store".to_string(),
            "Thumbs.db".to_string(),
            "*.swp".to_string(),
            "*.swo".to_string(),
            "*~".to_string(),
            "".to_string(),
            "# Editor directories and files".to_string(),
            "[editors]".to_string(),
            ".idea/".to_string(),
            ".vscode/".to_string(),
            "*.sublime-project".to_string(),
            "*.sublime-workspace".to_string(),
            "".to_string(),
        ];

        // Auto-detect project type and add relevant ignores
        if workdir.join("Cargo.toml").exists() {
            lines.extend_from_slice(&[
                "# Rust".to_string(),
                "[rust]".to_string(),
                "target/".to_string(),
                "*.rs.bk".to_string(),
                "".to_string(),
            ]);
        }

        if workdir.join("package.json").exists() {
            lines.extend_from_slice(&[
                "# Node.js".to_string(),
                "[node]".to_string(),
                "node_modules/".to_string(),
                "dist/".to_string(),
                "build/".to_string(),
                ".env".to_string(),
                ".env.local".to_string(),
                "*.log".to_string(),
                "".to_string(),
            ]);
        }

        if workdir.join("requirements.txt").exists()
            || workdir.join("setup.py").exists()
            || workdir.join("pyproject.toml").exists()
        {
            lines.extend_from_slice(&[
                "# Python".to_string(),
                "[python]".to_string(),
                "__pycache__/".to_string(),
                "*.py[cod]".to_string(),
                "*.egg-info/".to_string(),
                ".venv/".to_string(),
                "venv/".to_string(),
                "".to_string(),
            ]);
        }

        if workdir.join("go.mod").exists() {
            lines.extend_from_slice(&[
                "# Go".to_string(),
                "[go]".to_string(),
                "vendor/".to_string(),
                "".to_string(),
            ]);
        }

        // Always add large file filter at the end
        lines.extend_from_slice(&[
            "# Automatically ignore files larger than 50MB".to_string(),
            "size:>50mb".to_string(),
            "".to_string(),
        ]);

        lines.join("\n")
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_fgit_structure() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        assert!(repo.fgit_dir.exists());
        assert!(repo.fgit_dir.join("objects").exists());
        assert!(repo.fgit_dir.join("refs/heads").exists());
        assert!(repo.fgit_dir.join("refs/tags").exists());
        assert!(repo.fgit_dir.join("refs/remotes").exists());
        assert!(repo.fgit_dir.join("stash").exists());
        assert!(repo.fgit_dir.join("hooks").exists());
        assert!(repo.fgit_dir.join("logs").exists());
        assert!(repo.fgit_dir.join("HEAD").exists());
        assert!(repo.fgit_dir.join("config.toml").exists());
        assert!(repo.fgit_dir.join("index").exists());
        assert!(repo.fgit_dir.join("description").exists());
    }

    #[test]
    fn test_init_head_points_to_main() {
        let tmp = TempDir::new().unwrap();
        Repository::init(tmp.path()).unwrap();
        let head = fs::read_to_string(tmp.path().join(".fgit/HEAD")).unwrap();
        assert_eq!(head.trim(), "ref: refs/heads/main");
    }

    #[test]
    fn test_init_creates_fgitignore() {
        let tmp = TempDir::new().unwrap();
        Repository::init(tmp.path()).unwrap();
        assert!(tmp.path().join(".fgitignore").exists());
    }

    #[test]
    fn test_double_init_fails() {
        let tmp = TempDir::new().unwrap();
        Repository::init(tmp.path()).unwrap();
        assert!(Repository::init(tmp.path()).is_err());
    }

    #[test]
    fn test_open_existing_repo() {
        let tmp = TempDir::new().unwrap();
        Repository::init(tmp.path()).unwrap();
        let repo = Repository::open(tmp.path()).unwrap();
        assert_eq!(repo.workdir, tmp.path());
    }

    #[test]
    fn test_open_from_subdirectory() {
        let tmp = TempDir::new().unwrap();
        Repository::init(tmp.path()).unwrap();
        let sub = tmp.path().join("sub/deep");
        fs::create_dir_all(&sub).unwrap();
        let repo = Repository::open(&sub).unwrap();
        assert_eq!(repo.workdir, tmp.path());
    }

    #[test]
    fn test_store_and_read_blob() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        let blob = FgitObject::Blob(crate::object::Blob::new(b"hello fgit!".to_vec()));
        let hash = repo.store_object(&blob).unwrap();

        assert!(repo.object_exists(&hash));

        let read_back = repo.read_object(&hash).unwrap();
        assert_eq!(blob.hash(), read_back.hash());
    }

    #[test]
    fn test_hash_file() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        fs::write(tmp.path().join("test.txt"), "test content").unwrap();
        let hash = repo.hash_file(Path::new("test.txt")).unwrap();

        assert_eq!(hash.len(), 64);
        assert!(repo.object_exists(&hash));
    }

    #[test]
    fn test_auto_detect_rust_ignore() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), "[package]").unwrap();
        Repository::init(tmp.path()).unwrap();
        let ignore_content = fs::read_to_string(tmp.path().join(".fgitignore")).unwrap();
        assert!(ignore_content.contains("target/"));
    }
}
