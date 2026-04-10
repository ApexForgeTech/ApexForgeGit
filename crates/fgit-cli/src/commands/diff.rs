use fgit_core::{Repository, DiffEngine, DiffLineType};
use colored::Colorize;
use std::env;
use std::fs;

pub fn run(file: Option<&str>) -> Result<(), String> {
    let cwd = env::current_dir().map_err(|e| format!("{}", e))?;
    let repo = Repository::open(&cwd).map_err(|e| e.to_string())?;
    let index = repo.index().map_err(|e| e.to_string())?;

    let mut has_diff = false;

    for entry in &index.entries {
        if let Some(f) = file {
            if entry.path != f { continue; }
        }

        let file_path = repo.workdir.join(&entry.path);
        if !file_path.exists() {
            println!("  {} {} {}", "deleted:".bright_red(), entry.path, "(file removed)".dimmed());
            has_diff = true;
            continue;
        }

        let current = fs::read_to_string(&file_path).unwrap_or_default();
        let stored_obj = match repo.read_object(&entry.hash) {
            Ok(fgit_core::FgitObject::Blob(blob)) => {
                String::from_utf8(blob.content).unwrap_or_default()
            }
            _ => continue,
        };

        if current == stored_obj { continue; }

        has_diff = true;
        println!();
        println!("  {} {}", "━━━".bright_cyan(), entry.path.bright_white().bold());

        let hunks = DiffEngine::diff(&stored_obj, &current);
        for hunk in &hunks {
            println!("  {} {}", "@@ ".bright_cyan(),
                format!("-{},{} +{},{}", hunk.old_start, hunk.old_count,
                    hunk.new_start, hunk.new_count).bright_cyan());

            for line in &hunk.lines {
                match line.line_type {
                    DiffLineType::Added => {
                        println!("  {} {}", "+".bright_green(), line.content.bright_green());
                    }
                    DiffLineType::Removed => {
                        println!("  {} {}", "-".bright_red(), line.content.bright_red());
                    }
                    DiffLineType::Context => {
                        println!("  {} {}", " ".normal(), line.content.dimmed());
                    }
                }
            }
        }
    }

    if !has_diff {
        println!("  {} {}", "✓".bright_green(), "No differences found.".dimmed());
    }

    println!();
    Ok(())
}
