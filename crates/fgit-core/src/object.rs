use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fmt;

/// Object types in ApexForge Git (similar to Git but using SHA-256)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
    Tag,
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectType::Blob => write!(f, "blob"),
            ObjectType::Tree => write!(f, "tree"),
            ObjectType::Commit => write!(f, "commit"),
            ObjectType::Tag => write!(f, "tag"),
        }
    }
}

impl ObjectType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "blob" => Some(ObjectType::Blob),
            "tree" => Some(ObjectType::Tree),
            "commit" => Some(ObjectType::Commit),
            "tag" => Some(ObjectType::Tag),
            _ => None,
        }
    }
}

/// A blob object — stores raw file content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    pub content: Vec<u8>,
}

impl Blob {
    pub fn new(content: Vec<u8>) -> Self {
        Self { content }
    }

    pub fn size(&self) -> usize {
        self.content.len()
    }
}

/// File mode/permissions for tree entries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileMode {
    Regular,      // 100644
    Executable,   // 100755
    Symlink,      // 120000
    Directory,    // 040000
}

impl fmt::Display for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileMode::Regular => write!(f, "100644"),
            FileMode::Executable => write!(f, "100755"),
            FileMode::Symlink => write!(f, "120000"),
            FileMode::Directory => write!(f, "040000"),
        }
    }
}

impl FileMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "100644" => Some(FileMode::Regular),
            "100755" => Some(FileMode::Executable),
            "120000" => Some(FileMode::Symlink),
            "040000" => Some(FileMode::Directory),
            _ => None,
        }
    }
}

/// A single entry within a tree object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub mode: FileMode,
    pub name: String,
    pub hash: String,
}

/// A tree object — represents a directory snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add_entry(&mut self, mode: FileMode, name: String, hash: String) {
        self.entries.push(TreeEntry { mode, name, hash });
    }

    /// Sort entries alphabetically (directories first, then files)
    pub fn sort_entries(&mut self) {
        self.entries.sort_by(|a, b| {
            let a_is_dir = a.mode == FileMode::Directory;
            let b_is_dir = b.mode == FileMode::Directory;
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

/// Author/committer identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub name: String,
    pub email: String,
    pub timestamp: DateTime<Utc>,
}

impl Identity {
    pub fn new(name: String, email: String) -> Self {
        Self {
            name,
            email,
            timestamp: Utc::now(),
        }
    }

    pub fn with_timestamp(name: String, email: String, timestamp: DateTime<Utc>) -> Self {
        Self { name, email, timestamp }
    }
}

impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}> {}", self.name, self.email, self.timestamp.timestamp())
    }
}

/// A commit object — snapshot of the project at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub tree_hash: String,
    pub parent_hashes: Vec<String>,
    pub author: Identity,
    pub committer: Identity,
    pub message: String,
}

impl Commit {
    pub fn new(
        tree_hash: String,
        parent_hashes: Vec<String>,
        author: Identity,
        committer: Identity,
        message: String,
    ) -> Self {
        Self {
            tree_hash,
            parent_hashes,
            author,
            committer,
            message,
        }
    }

    /// Serialize commit to canonical byte representation for hashing
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut content = String::new();
        content.push_str(&format!("tree {}\n", self.tree_hash));
        for parent in &self.parent_hashes {
            content.push_str(&format!("parent {}\n", parent));
        }
        content.push_str(&format!("author {}\n", self.author));
        content.push_str(&format!("committer {}\n", self.committer));
        content.push_str(&format!("\n{}", self.message));
        content.into_bytes()
    }
}

/// Unified object wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FgitObject {
    Blob(Blob),
    Tree(Tree),
    Commit(Commit),
    Tag(crate::tag::Tag),
}

impl FgitObject {
    pub fn object_type(&self) -> ObjectType {
        match self {
            FgitObject::Blob(_) => ObjectType::Blob,
            FgitObject::Tree(_) => ObjectType::Tree,
            FgitObject::Commit(_) => ObjectType::Commit,
            FgitObject::Tag(_) => ObjectType::Tag,
        }
    }

    /// Serialize the object to bytes for hashing and storage
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            FgitObject::Blob(blob) => blob.content.clone(),
            FgitObject::Tree(tree) => {
                serde_json::to_vec(tree).expect("Failed to serialize tree")
            }
            FgitObject::Commit(commit) => commit.to_bytes(),
            FgitObject::Tag(tag) => {
                serde_json::to_vec(tag).expect("Failed to serialize tag")
            }
        }
    }

    /// Compute SHA-256 hash of the object (format: "<type> <size>\0<content>")
    pub fn hash(&self) -> String {
        let content = self.to_bytes();
        let header = format!("{} {}\0", self.object_type(), content.len());
        let mut hasher = Sha256::new();
        hasher.update(header.as_bytes());
        hasher.update(&content);
        hex::encode(hasher.finalize())
    }

    /// Serialize for storage (header + content)
    pub fn serialize(&self) -> Vec<u8> {
        let content = self.to_bytes();
        let header = format!("{} {}\0", self.object_type(), content.len());
        let mut result = header.into_bytes();
        result.extend_from_slice(&content);
        result
    }

    /// Deserialize from stored bytes
    pub fn deserialize(data: &[u8]) -> Result<Self, String> {
        // Find the null separator between header and content
        let null_pos = data.iter().position(|&b| b == 0)
            .ok_or_else(|| "Invalid object: no null separator found".to_string())?;

        let header = std::str::from_utf8(&data[..null_pos])
            .map_err(|e| format!("Invalid header encoding: {}", e))?;

        let parts: Vec<&str> = header.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err("Invalid object header format".to_string());
        }

        let obj_type = ObjectType::from_str(parts[0])
            .ok_or_else(|| format!("Unknown object type: {}", parts[0]))?;

        let content = &data[null_pos + 1..];

        match obj_type {
            ObjectType::Blob => {
                Ok(FgitObject::Blob(Blob::new(content.to_vec())))
            }
            ObjectType::Tree => {
                let tree: Tree = serde_json::from_slice(content)
                    .map_err(|e| format!("Failed to deserialize tree: {}", e))?;
                Ok(FgitObject::Tree(tree))
            }
            ObjectType::Commit => {
                // Parse commit from canonical text format
                let text = std::str::from_utf8(content)
                    .map_err(|e| format!("Invalid commit encoding: {}", e))?;
                let commit = parse_commit(text)?;
                Ok(FgitObject::Commit(commit))
            }
            ObjectType::Tag => {
                let tag: crate::tag::Tag = serde_json::from_slice(content)
                    .map_err(|e| format!("Failed to deserialize tag: {}", e))?;
                Ok(FgitObject::Tag(tag))
            }
        }
    }
}

/// Parse a commit from its canonical text representation
fn parse_commit(text: &str) -> Result<Commit, String> {
    let mut tree_hash = String::new();
    let mut parent_hashes = Vec::new();
    let mut author_str = String::new();
    let mut committer_str = String::new();
    let mut message_lines = Vec::new();
    let mut in_message = false;

    for line in text.lines() {
        if in_message {
            message_lines.push(line);
            continue;
        }

        if line.is_empty() {
            in_message = true;
            continue;
        }

        if let Some(rest) = line.strip_prefix("tree ") {
            tree_hash = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("parent ") {
            parent_hashes.push(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("author ") {
            author_str = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("committer ") {
            committer_str = rest.to_string();
        }
    }

    let author = parse_identity(&author_str)?;
    let committer = parse_identity(&committer_str)?;

    Ok(Commit {
        tree_hash,
        parent_hashes,
        author,
        committer,
        message: message_lines.join("\n"),
    })
}

/// Parse an identity string like "Name <email> timestamp"
fn parse_identity(s: &str) -> Result<Identity, String> {
    let lt_pos = s.find('<').ok_or("Invalid identity: missing <")?;
    let gt_pos = s.find('>').ok_or("Invalid identity: missing >")?;

    let name = s[..lt_pos].trim().to_string();
    let email = s[lt_pos + 1..gt_pos].to_string();
    let timestamp_str = s[gt_pos + 1..].trim();

    let timestamp = if timestamp_str.is_empty() {
        Utc::now()
    } else {
        let ts: i64 = timestamp_str.parse()
            .map_err(|_| format!("Invalid timestamp: {}", timestamp_str))?;
        DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
    };

    Ok(Identity { name, email, timestamp })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blob_hash_deterministic() {
        let blob1 = FgitObject::Blob(Blob::new(b"hello world".to_vec()));
        let blob2 = FgitObject::Blob(Blob::new(b"hello world".to_vec()));
        assert_eq!(blob1.hash(), blob2.hash());
    }

    #[test]
    fn test_blob_different_content_different_hash() {
        let blob1 = FgitObject::Blob(Blob::new(b"hello".to_vec()));
        let blob2 = FgitObject::Blob(Blob::new(b"world".to_vec()));
        assert_ne!(blob1.hash(), blob2.hash());
    }

    #[test]
    fn test_blob_hash_is_sha256() {
        let blob = FgitObject::Blob(Blob::new(b"test".to_vec()));
        let hash = blob.hash();
        assert_eq!(hash.len(), 64); // SHA-256 = 64 hex chars
    }

    #[test]
    fn test_blob_serialize_deserialize() {
        let original = FgitObject::Blob(Blob::new(b"hello fgit".to_vec()));
        let serialized = original.serialize();
        let deserialized = FgitObject::deserialize(&serialized).unwrap();
        assert_eq!(original.hash(), deserialized.hash());
    }

    #[test]
    fn test_tree_entries() {
        let mut tree = Tree::new();
        tree.add_entry(FileMode::Regular, "README.md".to_string(), "abc123".to_string());
        tree.add_entry(FileMode::Directory, "src".to_string(), "def456".to_string());
        tree.sort_entries();
        assert_eq!(tree.entries[0].name, "src"); // directories first
        assert_eq!(tree.entries[1].name, "README.md");
    }

    #[test]
    fn test_commit_serialize_deserialize() {
        let commit = Commit::new(
            "abc123def456".to_string(),
            vec![],
            Identity::new("Neo".to_string(), "neo@apexforge.dev".to_string()),
            Identity::new("Neo".to_string(), "neo@apexforge.dev".to_string()),
            "Initial commit".to_string(),
        );
        let obj = FgitObject::Commit(commit);
        let serialized = obj.serialize();
        let deserialized = FgitObject::deserialize(&serialized).unwrap();
        assert_eq!(obj.hash(), deserialized.hash());
    }

    #[test]
    fn test_object_type_display() {
        assert_eq!(ObjectType::Blob.to_string(), "blob");
        assert_eq!(ObjectType::Tree.to_string(), "tree");
        assert_eq!(ObjectType::Commit.to_string(), "commit");
        assert_eq!(ObjectType::Tag.to_string(), "tag");
    }
}
