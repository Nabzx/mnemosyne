//! `mnem`, the Mnemosyne command line interface.
//!
//! Phase 0 wires up the command tree and `--version`. The subcommands are
//! declared but not implemented; each one lands in its own phase. See
//! `ROADMAP.md`.

use clap::{Parser, Subcommand};

/// Version control for AI agent memory.
#[derive(Parser)]
#[command(name = "mnem", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new memory store in the current directory (Phase 1).
    Init,
    /// Stage a memory node for the next commit (Phase 1).
    Add,
    /// Record the staged memory state as a commit (Phase 1).
    Commit,
    /// Show the commit history (Phase 1).
    Log,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            println!("mnem {}", env!("CARGO_PKG_VERSION"));
            println!("Version control for AI agent memory. Run `mnem --help`.");
            Ok(())
        }
        Some(command) => {
            let name = match command {
                Command::Init => "init",
                Command::Add => "add",
                Command::Commit => "commit",
                Command::Log => "log",
            };
            anyhow::bail!("`mnem {name}` is not implemented yet. See ROADMAP.md.")
        }
    }
}
