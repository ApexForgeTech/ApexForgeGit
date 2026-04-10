mod commands;

use clap::{Parser, Subcommand};

/// ApexForge Git — Next-generation Version Control System
#[derive(Parser)]
#[command(name = "fgit", version, about = "ApexForge Git — Next-generation VCS", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new ApexForge Git repository
    Init,
    /// Save changes (stage all + commit in one step)
    Save {
        /// Commit message
        #[arg(short, long)]
        message: Option<String>,
        /// Interactive staging
        #[arg(short, long)]
        interactive: bool,
        /// Positional message (alternative to -m)
        #[arg(trailing_var_arg = true)]
        msg_words: Vec<String>,
    },
    /// Show working tree status
    Status,
    /// Show commit history with visual graph
    History {
        /// Number of commits to show
        #[arg(short = 'n', long, default_value = "20")]
        count: usize,
    },
    /// Show differences between working tree and index
    Diff {
        /// File to diff (optional, defaults to all)
        file: Option<String>,
    },
    /// Branch operations
    Branch {
        #[command(subcommand)]
        action: BranchAction,
    },
    /// Switch to a different branch
    Switch {
        /// Branch name to switch to
        name: String,
    },
    /// Merge a branch into the current branch
    Merge {
        /// Branch to merge
        name: String,
    },
    /// Undo the last commit (safe revert)
    Undo,
    /// Stash operations
    Stash {
        #[command(subcommand)]
        action: Option<StashAction>,
    },
    /// Tag operations
    Tag {
        /// Tag name
        name: Option<String>,
        /// Tag message (for annotated tags)
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Remote operations
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },
    /// Sync with remote (pull + push)
    Sync {
        /// Force push
        #[arg(short, long)]
        force: bool,
    },
    /// Clone a remote repository to local
    Clone {
        /// Remote URL or local path
        url: String,
        /// Target directory name
        dir: Option<String>,
    },
    /// Reset current HEAD to the specified state
    Reset {
        /// Commit hash or reference
        commit: String,
        /// Does not touch the index file or the working tree
        #[arg(long, group="mode")]
        soft: bool,
        /// Resets the index but not the working tree (default)
        #[arg(long, group="mode")]
        mixed: bool,
        /// Resets the index and working tree. Any changes are discarded
        #[arg(long, group="mode")]
        hard: bool,
    },
    /// Restore working tree files
    Restore {
        /// File path to restore
        path: String,
        /// Restore from a specific commit (defaults to HEAD)
        #[arg(short, long)]
        source: Option<String>,
        /// Restore into index as well (like `git reset <file>`)
        #[arg(short, long)]
        staged: bool,
    },
    /// Cleanup unreachable objects
    Gc,
    /// Rebase current branch on top of another branch
    Rebase {
        branch: String,
    },
}

#[derive(Subcommand)]
enum BranchAction {
    /// Create a new branch
    Create { name: String },
    /// Delete a branch
    Delete {
        name: String,
        /// Force delete unmerged branch
        #[arg(short='D', long)]
        force: bool,
    },
    /// List all branches
    List,
}

#[derive(Subcommand)]
enum StashAction {
    /// Pop the latest stash
    Pop,
    /// List all stashes
    List,
}

#[derive(Subcommand)]
enum RemoteAction {
    /// Add a remote
    Add { name: String, url: String },
    /// List remotes
    List,
    /// Remove a remote
    Remove { name: String },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Save { message, interactive, msg_words } => {
            let msg = message.unwrap_or_else(|| {
                if msg_words.is_empty() {
                    String::new()
                } else {
                    msg_words.join(" ")
                }
            });
            commands::save::run(&msg, interactive)
        }
        Commands::Status => commands::status::run(),
        Commands::History { count } => commands::history::run(count),
        Commands::Diff { file } => commands::diff::run(file.as_deref()),
        Commands::Branch { action } => match action {
            BranchAction::Create { name } => commands::branch::create(&name),
            BranchAction::Delete { name, force } => commands::branch::delete(&name, force),
            BranchAction::List => commands::branch::list(),
        },
        Commands::Switch { name } => commands::switch::run(&name),
        Commands::Merge { name } => commands::merge::run(&name),
        Commands::Undo => commands::undo::run(),
        Commands::Stash { action } => match action {
            Some(StashAction::Pop) => commands::stash::pop(),
            Some(StashAction::List) => commands::stash::list(),
            None => commands::stash::save(),
        },
        Commands::Tag { name, message } => commands::tag::run(name.as_deref(), message.as_deref()),
        Commands::Remote { action } => match action {
            RemoteAction::Add { name, url } => commands::remote::add(&name, &url),
            RemoteAction::List => commands::remote::list(),
            RemoteAction::Remove { name } => commands::remote::remove(&name),
        },
        Commands::Sync { force } => commands::sync::run(force),
        Commands::Clone { url, dir } => commands::clone::run(&url, dir.as_deref()),
        Commands::Reset { commit, soft, mixed, hard } => {
            let mode = if hard {
                "hard"
            } else if soft {
                "soft"
            } else {
                "mixed"
            };
            commands::reset::run(&commit, mode)
        }
        Commands::Restore { path, source, staged } => {
            commands::restore::run(&path, source.as_deref(), staged)
        }
        Commands::Gc => commands::gc::run(),
        Commands::Rebase { branch } => commands::rebase::run(&branch),
    };

    if let Err(e) = result {
        eprintln!("\x1b[1;31m✗ Error:\x1b[0m {}", e);
        std::process::exit(1);
    }
}
