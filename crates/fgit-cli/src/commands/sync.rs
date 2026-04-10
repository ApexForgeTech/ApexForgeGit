use colored::Colorize;
use fgit_core::Repository;
use fgit_core::network::NetworkEngine;
use std::env;
use std::path::Path;

pub fn run(force: bool) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    
    let config = repo.config().map_err(|e| e.to_string())?;
    
    // Find origin remote
    let origin_url = config.remote.get("origin")
        .map(|r| r.url.clone());
        
    if let Some(url) = origin_url {
        println!("  {} Syncing with remote 'origin' -> {}", "ℹ".bright_blue(), url.dimmed());
        let remote_path = Path::new(&url);
        
        // Push
        match NetworkEngine::push(&repo, remote_path) {
            Ok(_) => println!("    {} Pushed objects & refs", "↑".bright_green()),
            Err(e) => println!("    {} Push failed: {}", "✗".bright_red(), e),
        }
        
        // Pull
        match NetworkEngine::pull(&repo, remote_path) {
            Ok(_) => println!("    {} Pulled objects & refs", "↓".bright_green()),
            Err(e) => println!("    {} Pull failed: {}", "✗".bright_red(), e),
        }
    } else {
        println!("  {} {}", "✗".bright_red(), "No remote named 'origin' configured. Use `fgit remote add origin <path>`.".bright_yellow());
    }

    Ok(())
}
