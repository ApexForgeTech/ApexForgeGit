use std::process::Command;
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;

/// Hook manager that executes scripts from the `.fgit/hooks/` directory
pub struct HookManager {
    hooks_dir: PathBuf,
}

impl HookManager {
    pub fn new(fgit_dir: &Path) -> Self {
        Self {
            hooks_dir: fgit_dir.join("hooks"),
        }
    }

    /// Run a specific hook (e.g., "pre-save"). 
    /// Returns Ok if hook doesn't exist, is not executable, or exits with status 0.
    /// Returns Err with message if hook fails (non-zero exit).
    pub fn run(&self, hook_name: &str) -> Result<(), String> {
        let hook_path = self.hooks_dir.join(hook_name);

        if !hook_path.exists() {
            return Ok(());
        }

        let metadata = std::fs::metadata(&hook_path)
            .map_err(|e| format!("Could not read hook '{}': {}", hook_name, e))?;

        if metadata.permissions().mode() & 0o111 == 0 {
            // Hook exists but is not executable, ignore
            return Ok(());
        }

        println!("Running hook: {}...", hook_name);
        
        let status = Command::new(&hook_path)
            .status()
            .map_err(|e| format!("Failed to execute hook '{}': {}", hook_name, e))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "Hook '{}' failed with status: {}",
                hook_name,
                status.code().unwrap_or(-1)
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_hooks_missing_gives_ok() {
        let tmp = TempDir::new().unwrap();
        let fgit_dir = tmp.path().join(".fgit");
        let mgr = HookManager::new(&fgit_dir);
        assert!(mgr.run("pre-save").is_ok());
    }

    #[test]
    fn test_hooks_success_script() {
        let tmp = TempDir::new().unwrap();
        let fgit_dir = tmp.path().join(".fgit");
        let hooks_dir = fgit_dir.join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();

        let script = hooks_dir.join("pre-save");
        fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();
        
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();

        let mgr = HookManager::new(&fgit_dir);
        assert!(mgr.run("pre-save").is_ok());
    }

    #[test]
    fn test_hooks_failure_script() {
        let tmp = TempDir::new().unwrap();
        let fgit_dir = tmp.path().join(".fgit");
        let hooks_dir = fgit_dir.join("hooks");
        fs::create_dir_all(&hooks_dir).unwrap();

        let script = hooks_dir.join("pre-save");
        fs::write(&script, "#!/bin/sh\nexit 1\n").unwrap();
        
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();

        let mgr = HookManager::new(&fgit_dir);
        let result = mgr.run("pre-save");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed with status: 1"));
    }
}
