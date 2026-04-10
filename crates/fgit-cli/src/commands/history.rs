use fgit_core::Repository;
use colored::Colorize;
use std::env;

pub fn run(count: usize) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let refs = repo.refs();

    let branch = refs.current_branch().unwrap_or(None)
        .unwrap_or_else(|| "detached".to_string());

    let mut current_hash = refs.resolve_head().map_err(|e| e)?;

    println!();
    println!("  {} {}", "Branch:".dimmed(), branch.bright_cyan().bold());
    println!("  {}", "─".repeat(50).dimmed());

    let mut shown = 0;
    while let Some(hash) = current_hash {
        if shown >= count { break; }

        let obj = match repo.read_object(&hash) {
            Ok(o) => o,
            Err(_) => break,
        };

        if let fgit_core::FgitObject::Commit(commit) = obj {
            let short = &hash[..8];
            let time = commit.author.timestamp.format("%Y-%m-%d %H:%M");

            // Graph decoration
            let is_head = shown == 0;
            let marker = if is_head { "●".bright_green().bold() } else { "○".bright_blue() };

            println!("  {} {} {}",
                marker,
                short.bright_yellow(),
                commit.message.bright_white()
            );
            println!("  {} {} {} {}",
                "│".dimmed(),
                commit.author.name.dimmed(),
                "<".dimmed(),
                format!("{}>", commit.author.email).dimmed()
            );
            println!("  {} {}",
                "│".dimmed(),
                time.to_string().dimmed()
            );

            if commit.parent_hashes.len() > 1 {
                println!("  {} {}", "│".dimmed(), "⤴ merge commit".bright_magenta());
            }

            println!("  {}", "│".dimmed());

            current_hash = commit.parent_hashes.first().cloned();
            shown += 1;
        } else {
            break;
        }
    }

    if shown == 0 {
        println!("  {} {}", "ℹ".bright_blue(), "No commits yet.".dimmed());
    }

    println!();
    Ok(())
}
