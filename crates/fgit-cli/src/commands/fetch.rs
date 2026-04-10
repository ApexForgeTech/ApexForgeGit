use std::env;
use std::path::Path;
use colored::Colorize;
use fgit_core::Repository;
use fgit_core::network::NetworkEngine;

pub fn run() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    
    let config = repo.config().map_err(|e| e.to_string())?;
    
    let origin_url = config.remote.get("origin").map(|r| r.url.clone());
        
    if let Some(url) = origin_url {
        println!("  {} Fetching from remote 'origin' -> {}", "ℹ".bright_blue(), url.dimmed());
        let remote_path = Path::new(&url);
        
        match NetworkEngine::pull(&repo, remote_path) {
            Ok(_) => println!("    {} Downloaded objects & refs successfully. Working tree unchanged.", "✓".bright_green()),
            Err(e) => println!("    {} Fetch failed: {}", "✗".bright_red(), e),
        }
    } else {
        println!("  {} {}", "✗".bright_red(), "No remote named 'origin' configured.".bright_yellow());
    }

    Ok(())
}
