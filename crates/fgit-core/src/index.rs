use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexEntry {
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub mtime: i64,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    pub version: u32,
    pub entries: Vec<IndexEntry>,
}

impl Index {
    pub fn new() -> Self {
        Self { version: 1, entries: Vec::new() }
    }

    pub fn add(&mut self, entry: IndexEntry) {
        self.entries.retain(|e| e.path != entry.path);
        self.entries.push(entry);
        self.entries.sort_by(|a, b| a.path.cmp(&b.path));
    }

    pub fn remove(&mut self, path: &str) -> bool {
        let len_before = self.entries.len();
        self.entries.retain(|e| e.path != path);
        self.entries.len() < len_before
    }

    pub fn get(&self, path: &str) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.path == path)
    }

    pub fn contains(&self, path: &str) -> bool {
        self.entries.iter().any(|e| e.path == path)
    }

    pub fn paths(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.path.as_str()).collect()
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }
    pub fn clear(&mut self) { self.entries.clear(); }

    pub fn entries_by_directory(&self) -> BTreeMap<String, Vec<&IndexEntry>> {
        let mut map: BTreeMap<String, Vec<&IndexEntry>> = BTreeMap::new();
        for entry in &self.entries {
            let dir = if let Some(pos) = entry.path.find('/') {
                entry.path[..pos].to_string()
            } else {
                ".".to_string()
            };
            map.entry(dir).or_default().push(entry);
        }
        map
    }

    pub fn diff_with(&self, other: &Index) -> IndexDiff {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let mut deleted = Vec::new();

        let self_map: BTreeMap<&str, &IndexEntry> =
            self.entries.iter().map(|e| (e.path.as_str(), e)).collect();
        let other_map: BTreeMap<&str, &IndexEntry> =
            other.entries.iter().map(|e| (e.path.as_str(), e)).collect();

        for (path, entry) in &other_map {
            match self_map.get(path) {
                None => added.push((*path).to_string()),
                Some(old) => {
                    if old.hash != entry.hash {
                        modified.push((*path).to_string());
                    }
                }
            }
        }
        for path in self_map.keys() {
            if !other_map.contains_key(path) {
                deleted.push((*path).to_string());
            }
        }

        IndexDiff { added, modified, deleted }
    }
}

impl Default for Index {
    fn default() -> Self { Self::new() }
}

#[derive(Debug, Clone)]
pub struct IndexDiff {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub deleted: Vec<String>,
}

impl IndexDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(path: &str, hash: &str) -> IndexEntry {
        IndexEntry {
            path: path.to_string(), hash: hash.to_string(),
            size: 100, mtime: 1234567890, mode: "100644".to_string(),
        }
    }

    #[test]
    fn test_add_and_get() {
        let mut idx = Index::new();
        idx.add(make_entry("src/main.rs", "abc123"));
        assert_eq!(idx.len(), 1);
        assert!(idx.contains("src/main.rs"));
    }

    #[test]
    fn test_update_existing() {
        let mut idx = Index::new();
        idx.add(make_entry("f.txt", "h1"));
        idx.add(make_entry("f.txt", "h2"));
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.get("f.txt").unwrap().hash, "h2");
    }

    #[test]
    fn test_remove() {
        let mut idx = Index::new();
        idx.add(make_entry("f.txt", "h1"));
        assert!(idx.remove("f.txt"));
        assert!(!idx.contains("f.txt"));
    }

    #[test]
    fn test_diff_with() {
        let mut old = Index::new();
        old.add(make_entry("keep.txt", "same"));
        old.add(make_entry("mod.txt", "old"));
        old.add(make_entry("del.txt", "del"));

        let mut new = Index::new();
        new.add(make_entry("keep.txt", "same"));
        new.add(make_entry("mod.txt", "new"));
        new.add(make_entry("add.txt", "add"));

        let diff = old.diff_with(&new);
        assert_eq!(diff.added, vec!["add.txt"]);
        assert_eq!(diff.modified, vec!["mod.txt"]);
        assert_eq!(diff.deleted, vec!["del.txt"]);
    }
}
