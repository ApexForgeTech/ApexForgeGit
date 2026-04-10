use serde::{Serialize, Deserialize};

/// ApexForge Git configuration — stored as .fgit/config.toml
/// Much cleaner and more readable than Git's .ini-style config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FgitConfig {
    pub user: UserConfig,
    pub core: CoreConfig,
    #[serde(default)]
    pub remote: std::collections::HashMap<String, RemoteConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// Hash algorithm: "sha256" (only option for now, future-proof)
    pub hash_algorithm: String,
    /// Compression algorithm: "zstd"
    pub compression: String,
    /// Default branch name
    pub default_branch: String,
    /// Enable auto-sync mode
    pub auto_sync: bool,
    /// File size warning threshold in bytes (warn when committing large files)
    pub large_file_threshold: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub url: String,
    #[serde(default)]
    pub fetch: Option<String>,
}

impl Default for FgitConfig {
    fn default() -> Self {
        Self {
            user: UserConfig {
                name: String::new(),
                email: String::new(),
            },
            core: CoreConfig {
                hash_algorithm: "sha256".to_string(),
                compression: "zstd".to_string(),
                default_branch: "main".to_string(),
                auto_sync: false,
                large_file_threshold: 50 * 1024 * 1024, // 50MB
            },
            remote: std::collections::HashMap::new(),
        }
    }
}

impl FgitConfig {
    /// Update user name and email
    pub fn set_user(&mut self, name: String, email: String) {
        self.user.name = name;
        self.user.email = email;
    }

    /// Add or update a remote
    pub fn add_remote(&mut self, name: String, url: String) {
        self.remote.insert(name, RemoteConfig { url, fetch: None });
    }

    /// Remove a remote
    pub fn remove_remote(&mut self, name: &str) -> bool {
        self.remote.remove(name).is_some()
    }

    /// Get a remote by name
    pub fn get_remote(&self, name: &str) -> Option<&RemoteConfig> {
        self.remote.get(name)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FgitConfig::default();
        assert_eq!(config.core.hash_algorithm, "sha256");
        assert_eq!(config.core.compression, "zstd");
        assert_eq!(config.core.default_branch, "main");
        assert!(!config.core.auto_sync);
    }

    #[test]
    fn test_config_serialize_deserialize() {
        let mut config = FgitConfig::default();
        config.set_user("Neo".to_string(), "neo@apexforge.dev".to_string());
        config.add_remote("origin".to_string(), "https://forge.dev/repo".to_string());

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: FgitConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.user.name, "Neo");
        assert_eq!(parsed.user.email, "neo@apexforge.dev");
        assert!(parsed.remote.contains_key("origin"));
    }

    #[test]
    fn test_add_remove_remote() {
        let mut config = FgitConfig::default();
        config.add_remote("origin".to_string(), "https://example.com".to_string());
        assert!(config.get_remote("origin").is_some());

        config.remove_remote("origin");
        assert!(config.get_remote("origin").is_none());
    }
}
