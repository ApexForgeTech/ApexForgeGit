use regex::Regex;
use std::path::Path;

/// A single ignore rule parsed from .fgitignore
#[derive(Debug, Clone)]
pub enum IgnoreRule {
    /// Standard glob pattern (e.g., "*.log", "target/")
    Glob { pattern: String, negated: bool },
    /// Regex pattern (prefixed with "regex:" in .fgitignore)
    Regex { pattern: Regex, negated: bool },
    /// Size-based filter (e.g., "size:>10mb")
    Size { max_bytes: u64 },
}

/// The section a rule belongs to (for organized .fgitignore)
#[derive(Debug, Clone)]
pub struct SectionedRule {
    pub section: Option<String>,
    pub rule: IgnoreRule,
}

/// Parsed .fgitignore rules
#[derive(Debug)]
pub struct IgnoreRules {
    pub rules: Vec<SectionedRule>,
}

impl IgnoreRules {
    /// Parse a .fgitignore file content into ignore rules
    pub fn parse(content: &str) -> Self {
        let mut rules = Vec::new();
        let mut current_section: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Section headers: [section_name]
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = Some(trimmed[1..trimmed.len() - 1].to_string());
                continue;
            }

            // Size filter: size:>10mb, size:>500kb
            if let Some(size_spec) = trimmed.strip_prefix("size:>") {
                if let Some(bytes) = parse_size(size_spec) {
                    rules.push(SectionedRule {
                        section: current_section.clone(),
                        rule: IgnoreRule::Size { max_bytes: bytes },
                    });
                }
                continue;
            }

            // Regex pattern: regex:<pattern>
            if let Some(regex_str) = trimmed.strip_prefix("regex:") {
                let (negated, pattern_str) = if let Some(p) = regex_str.strip_prefix('!') {
                    (true, p)
                } else {
                    (false, regex_str)
                };

                if let Ok(regex) = Regex::new(pattern_str) {
                    rules.push(SectionedRule {
                        section: current_section.clone(),
                        rule: IgnoreRule::Regex {
                            pattern: regex,
                            negated,
                        },
                    });
                }
                continue;
            }

            // Standard glob pattern (possibly negated with !)
            let (negated, pattern) = if let Some(p) = trimmed.strip_prefix('!') {
                (true, p.to_string())
            } else {
                (false, trimmed.to_string())
            };

            rules.push(SectionedRule {
                section: current_section.clone(),
                rule: IgnoreRule::Glob { pattern, negated },
            });
        }

        Self { rules }
    }

    /// Create empty ignore rules
    pub fn empty() -> Self {
        Self { rules: Vec::new() }
    }

    /// Check if a file path should be ignored
    pub fn is_ignored(&self, path: &str, file_size: Option<u64>) -> bool {
        let mut ignored = false;

        for sectioned_rule in &self.rules {
            match &sectioned_rule.rule {
                IgnoreRule::Glob { pattern, negated } => {
                    if glob_matches(pattern, path) {
                        if *negated {
                            ignored = false; // Un-ignore (negate)
                        } else {
                            ignored = true;
                        }
                    }
                }
                IgnoreRule::Regex { pattern, negated } => {
                    if pattern.is_match(path) {
                        if *negated {
                            ignored = false;
                        } else {
                            ignored = true;
                        }
                    }
                }
                IgnoreRule::Size { max_bytes } => {
                    if let Some(size) = file_size {
                        if size > *max_bytes {
                            ignored = true;
                        }
                    }
                }
            }
        }

        ignored
    }
}

/// Parse human-readable sizes like "10mb", "500kb", "1gb"
fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim().to_lowercase();

    if let Some(num_str) = s.strip_suffix("gb") {
        num_str.trim().parse::<u64>().ok().map(|n| n * 1024 * 1024 * 1024)
    } else if let Some(num_str) = s.strip_suffix("mb") {
        num_str.trim().parse::<u64>().ok().map(|n| n * 1024 * 1024)
    } else if let Some(num_str) = s.strip_suffix("kb") {
        num_str.trim().parse::<u64>().ok().map(|n| n * 1024)
    } else if let Some(num_str) = s.strip_suffix('b') {
        num_str.trim().parse::<u64>().ok()
    } else {
        s.parse::<u64>().ok()
    }
}

/// Simple glob matching supporting *, **, and /
fn glob_matches(pattern: &str, path: &str) -> bool {
    // Handle directory patterns (ending with /)
    let (pattern, match_dir) = if pattern.ends_with('/') {
        (&pattern[..pattern.len() - 1], true)
    } else {
        (pattern, false)
    };

    if match_dir {
        // Check if the path IS the directory or is inside it
        let p = Path::new(path);
        for ancestor in p.ancestors() {
            let ancestor_str = ancestor.to_str().unwrap_or("");
            if !ancestor_str.is_empty() && simple_glob_match(pattern, ancestor_str) {
                return true;
            }
        }
        return false;
    }

    // Check against full path and just the filename
    if simple_glob_match(pattern, path) {
        return true;
    }

    // Also try matching just the filename component
    if let Some(filename) = Path::new(path).file_name().and_then(|f| f.to_str()) {
        if simple_glob_match(pattern, filename) {
            return true;
        }
    }

    false
}

/// Very simple glob matching (* = any chars except /, ** = any path segment)
fn simple_glob_match(pattern: &str, text: &str) -> bool {
    // Convert glob pattern to regex
    let mut regex_str = String::from("^");

    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '*' if i + 1 < chars.len() && chars[i + 1] == '*' => {
                regex_str.push_str(".*");
                i += 2;
                if i < chars.len() && chars[i] == '/' {
                    i += 1; // skip the / after **
                }
            }
            '*' => {
                regex_str.push_str("[^/]*");
                i += 1;
            }
            '?' => {
                regex_str.push_str("[^/]");
                i += 1;
            }
            '.' => {
                regex_str.push_str("\\.");
                i += 1;
            }
            c => {
                if regex::escape(&c.to_string()) != c.to_string() {
                    regex_str.push_str(&regex::escape(&c.to_string()));
                } else {
                    regex_str.push(c);
                }
                i += 1;
            }
        }
    }
    regex_str.push('$');

    if let Ok(re) = Regex::new(&regex_str) {
        re.is_match(text)
    } else {
        false
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_pattern() {
        let rules = IgnoreRules::parse("*.log\ntarget/\n");
        assert!(rules.is_ignored("error.log", None));
        assert!(rules.is_ignored("target/debug/main", None));
        assert!(!rules.is_ignored("src/main.rs", None));
    }

    #[test]
    fn test_negation() {
        let rules = IgnoreRules::parse("*.log\n!important.log\n");
        assert!(rules.is_ignored("debug.log", None));
        assert!(!rules.is_ignored("important.log", None));
    }

    #[test]
    fn test_regex_pattern() {
        let rules = IgnoreRules::parse("regex:^test_.*\\.tmp$\n");
        assert!(rules.is_ignored("test_output.tmp", None));
        assert!(!rules.is_ignored("production.tmp", None));
    }

    #[test]
    fn test_size_filter() {
        let rules = IgnoreRules::parse("size:>10mb\n");
        let big = Some(20 * 1024 * 1024); // 20MB
        let small = Some(5 * 1024 * 1024); // 5MB
        assert!(rules.is_ignored("big_file.bin", big));
        assert!(!rules.is_ignored("small_file.txt", small));
    }

    #[test]
    fn test_sections_are_parsed() {
        let content = "[build]\ntarget/\n*.o\n\n[deps]\nnode_modules/\n";
        let rules = IgnoreRules::parse(content);
        assert_eq!(rules.rules.len(), 3);
        assert_eq!(rules.rules[0].section, Some("build".to_string()));
        assert_eq!(rules.rules[1].section, Some("build".to_string()));
        assert_eq!(rules.rules[2].section, Some("deps".to_string()));
    }

    #[test]
    fn test_comments_and_empty_lines() {
        let content = "# This is a comment\n\n*.log\n  # Another comment\n";
        let rules = IgnoreRules::parse(content);
        assert_eq!(rules.rules.len(), 1);
    }

    #[test]
    fn test_parse_size_units() {
        assert_eq!(parse_size("10mb"), Some(10 * 1024 * 1024));
        assert_eq!(parse_size("500kb"), Some(500 * 1024));
        assert_eq!(parse_size("1gb"), Some(1024 * 1024 * 1024));
        assert_eq!(parse_size("1024b"), Some(1024));
    }

    #[test]
    fn test_directory_pattern_matching() {
        let rules = IgnoreRules::parse("node_modules/\n");
        assert!(rules.is_ignored("node_modules/express/index.js", None));
        assert!(!rules.is_ignored("my_modules/test.js", None));
    }
}
