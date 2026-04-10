use colored::Colorize;
use fgit_core::network::NetworkEngine;
use std::path::{Path, PathBuf};
use std::env;

pub fn run(url: &str, dir: Option<&str>) -> Result<(), String> {
    let source_path = Path::new(url);
    
    let target_name = match dir {
        Some(d) => d.to_string(),
        None => source_path.file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("cloned_repo"))
            .to_string_lossy()
            .into_owned()
    };
    
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let target_path = cwd.join(&target_name);

    println!("  {} Cloning {} into {}...", "ℹ".bright_blue(), url.bright_white(), target_name.bright_cyan());

    match NetworkEngine::clone_local(source_path, &target_path) {
        Ok(_) => {
            println!("  {} Clone successful", "✓".bright_green());
            Ok(())
        }
        Err(e) => Err(format!("Clone failed: {}", e))
    }
}
