use fgit_core::Repository;
use colored::Colorize;
use std::env;

pub fn run() -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("Cannot get cwd: {}", e))?;
    let repo = Repository::init(&cwd).map_err(|e| e.to_string())?;

    println!("{}", "╔══════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║     ApexForge Git — Repository Initialized      ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════╝".bright_cyan());
    println!();
    println!("  {} {}", "📂 Location:".bright_white(), repo.workdir.display());
    println!("  {} {}", "🔧 Config:".bright_white(), ".fgit/config.toml");
    println!("  {} {}", "📋 Ignore:".bright_white(), ".fgitignore (auto-generated)");
    println!("  {} {}", "🌿 Branch:".bright_white(), "main".bright_green());
    println!();
    println!("  {} {}", "→".bright_yellow(), "Start with: fgit save \"Initial commit\"".dimmed());

    Ok(())
}
