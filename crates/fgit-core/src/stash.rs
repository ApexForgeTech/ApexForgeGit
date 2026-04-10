use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;

/// A stash entry — stored in .fgit/stash/ as individual JSON files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    pub id: u32,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub changes: Vec<StashedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashedFile {
    pub path: String,
    pub content: Vec<u8>,
    pub original_hash: String,
}

pub struct StashManager {
    stash_dir: PathBuf,
}

impl StashManager {
    pub fn new(fgit_dir: &PathBuf) -> Self {
        Self { stash_dir: fgit_dir.join("stash") }
    }

    pub fn save(&self, message: &str, changes: Vec<StashedFile>) -> Result<u32, String> {
        let id = self.next_id()?;
        let entry = StashEntry {
            id, message: message.to_string(),
            timestamp: Utc::now(), changes,
        };
        let data = serde_json::to_vec_pretty(&entry)
            .map_err(|e| format!("Serialize error: {}", e))?;
        fs::create_dir_all(&self.stash_dir)
            .map_err(|e| format!("Create stash dir: {}", e))?;
        fs::write(self.stash_dir.join(format!("{}.json", id)), data)
            .map_err(|e| format!("Write stash: {}", e))?;
        Ok(id)
    }

    pub fn pop(&self) -> Result<StashEntry, String> {
        let latest = self.latest_id()?
            .ok_or("No stash entries found")?;
        let path = self.stash_dir.join(format!("{}.json", latest));
        let data = fs::read(&path).map_err(|e| format!("Read stash: {}", e))?;
        let entry: StashEntry = serde_json::from_slice(&data)
            .map_err(|e| format!("Parse stash: {}", e))?;
        fs::remove_file(&path).map_err(|e| format!("Remove stash: {}", e))?;
        Ok(entry)
    }

    pub fn list(&self) -> Result<Vec<StashEntry>, String> {
        if !self.stash_dir.exists() { return Ok(Vec::new()); }
        let mut entries = Vec::new();
        let dir = fs::read_dir(&self.stash_dir)
            .map_err(|e| format!("Read stash dir: {}", e))?;
        for item in dir {
            let item = item.map_err(|e| format!("Dir entry: {}", e))?;
            if item.path().extension().map_or(false, |e| e == "json") {
                let data = fs::read(item.path())
                    .map_err(|e| format!("Read: {}", e))?;
                if let Ok(entry) = serde_json::from_slice::<StashEntry>(&data) {
                    entries.push(entry);
                }
            }
        }
        entries.sort_by_key(|e| e.id);
        Ok(entries)
    }

    fn next_id(&self) -> Result<u32, String> {
        Ok(self.latest_id()?.map_or(0, |id| id + 1))
    }

    fn latest_id(&self) -> Result<Option<u32>, String> {
        if !self.stash_dir.exists() { return Ok(None); }
        let mut max_id: Option<u32> = None;
        let dir = fs::read_dir(&self.stash_dir)
            .map_err(|e| format!("Read stash dir: {}", e))?;
        for item in dir {
            let item = item.map_err(|e| format!("Dir entry: {}", e))?;
            if let Some(name) = item.file_name().to_str() {
                if let Some(id_str) = name.strip_suffix(".json") {
                    if let Ok(id) = id_str.parse::<u32>() {
                        max_id = Some(max_id.map_or(id, |m: u32| m.max(id)));
                    }
                }
            }
        }
        Ok(max_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_list() {
        let tmp = TempDir::new().unwrap();
        let fgit = tmp.path().join(".fgit");
        fs::create_dir_all(&fgit).unwrap();
        let mgr = StashManager::new(&fgit);
        mgr.save("test stash", vec![]).unwrap();
        let list = mgr.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].message, "test stash");
    }

    #[test]
    fn test_save_and_pop() {
        let tmp = TempDir::new().unwrap();
        let fgit = tmp.path().join(".fgit");
        fs::create_dir_all(&fgit).unwrap();
        let mgr = StashManager::new(&fgit);
        mgr.save("first", vec![]).unwrap();
        mgr.save("second", vec![]).unwrap();
        let popped = mgr.pop().unwrap();
        assert_eq!(popped.message, "second");
        assert_eq!(mgr.list().unwrap().len(), 1);
    }
}
