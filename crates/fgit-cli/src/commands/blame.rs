use std::env;
use std::fs;
use colored::Colorize;
use fgit_core::{Repository, FgitObject};
use chrono::{DateTime, Utc, TimeZone};

pub fn run(file_path: &str) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    
    // Read local file or index to get current lines
    let abs_path = repo.workdir.join(file_path);
    let content = match fs::read_to_string(&abs_path) {
        Ok(c) => c,
        Err(_) => return Err(format!("File '{}' not found in working directory", file_path)),
    };
    
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        println!("  {} File is empty", "ℹ".bright_blue());
        return Ok(());
    }

    // Traverse history to find when the file was last modified
    // For MVP blame, we attribute the entire file to the commit that last changed it.
    // Full line-by-line blame requires full Myers Edit Graph tracing traversing N commits.
    
    let current_hash = match repo.refs().resolve_head().map_err(|e| e.to_string())? {
        Some(h) => h,
        None => return Err("No commits yet".to_string()),
    };

    let mut curr = current_hash.clone();
    let mut last_modifier = None;
    
    while let Ok(fgit_core::object::FgitObject::Commit(c)) = repo.read_object(&curr) {
        // Did the file change here?
        // We can just attribute to the latest commit for MVP since we don't have tree diff fully exposed.
        last_modifier = Some(c);
        break; 
        
        // MVP simplified approximation: We attribute everything to the latest commit that touches it.
    }

    if let Some(c) = last_modifier {
        let formatted_time = c.author.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        
        let author_str = format!("{} <{}>", c.author.name, c.author.email);
        let commit_short = &curr[0..8];
        
        println!("{:<10} {:<30} {:<20} | {}", "COMMIT".bold(), "AUTHOR".bold(), "DATE".bold(), "LINE".bold());
        println!("{}", "-".repeat(80));
        
        for (i, line) in lines.iter().enumerate() {
            println!("{:<10} {:<30} {:<20} | {} {}", 
                commit_short.bright_yellow(),
                author_str.bright_cyan(),
                formatted_time.bright_black(),
                format!("{:>4})", i + 1).dimmed(),
                line
            );
        }
    } else {
        println!("  {} Uncommitted / Untracked file", "⚠".bright_yellow());
    }

    Ok(())
}
