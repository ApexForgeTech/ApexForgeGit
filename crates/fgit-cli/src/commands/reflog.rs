use std::env;
use std::fs;
use colored::Colorize;
use fgit_core::Repository;

pub fn run() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;

    let log_path = repo.fgit_dir.join("logs/HEAD");
    if !log_path.exists() {
        println!("  {} No reflog history exists yet.", "ℹ".bright_blue());
        return Ok(());
    }

    let content = fs::read_to_string(&log_path).map_err(|e| format!("Failed to read reflog: {}", e))?;
    let mut lines: Vec<&str> = content.lines().collect();
    
    // Reverse because we want newest first
    lines.reverse();

    for (i, log) in lines.iter().enumerate() {
        // Format: OLD NEW TIMESTAMP MESSAGE (TIMESTAMP HAS SPACES LIKE YYYY-MM-DDTHH:MM...)
        let parts: Vec<&str> = log.splitn(4, ' ').collect();
        if parts.len() < 4 { continue; } // Malformed line
        
        let new_hash = parts[1];
        let msg = parts[3];

        let short_hash = if new_hash.len() >= 8 { &new_hash[..8] } else { new_hash };

        println!("{} {} {}: {}", 
            short_hash.bright_yellow(),
            format!("HEAD@{{{}}}", i).bright_green(),
            "commit".bright_magenta(),
            msg
        );
    }

    Ok(())
}
