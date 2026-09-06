//! `mnem`, the Mnemosyne command line interface.
//!
//! Four commands so far, each a thin wrapper over `mnem-core` (ADR-0004):
//! `init`, `add`, `commit`, `log`. See `ROADMAP.md`.

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mnem_core::{ContentKind, MemoryNode, ObjectId, Provenance, Store};

/// Version control for AI agent memory.
#[derive(Parser)]
#[command(name = "mnem", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new memory store in a directory (default: the current one).
    Init {
        /// Where to create the store.
        path: Option<PathBuf>,
    },
    /// Stage a memory node for the next commit.
    Add {
        /// The node's stable id. Staging the same id again is an update.
        id: String,
        /// The content, as a plain string. Omit to read it from stdin.
        content: Option<String>,
        /// Read the content from this file, parsed as JSON.
        #[arg(long, value_name = "FILE", conflicts_with = "content")]
        content_file: Option<PathBuf>,
        /// Provenance: which step of the run produced this.
        #[arg(long)]
        step: Option<String>,
        /// Provenance: an external source identifier.
        #[arg(long)]
        source: Option<String>,
        /// When the agent formed this, in Unix milliseconds.
        #[arg(long, value_name = "MS")]
        event_time: Option<i64>,
    },
    /// Record the staged memory state as a commit.
    Commit {
        /// The commit message.
        #[arg(short, long)]
        message: String,
        /// Who or what made the commit. Falls back to $MNEM_AUTHOR, then "unknown".
        #[arg(long)]
        author: Option<String>,
    },
    /// Show the commit history from HEAD, newest first.
    Log {
        /// Follow only the first parent of each commit.
        #[arg(long)]
        first_parent: bool,
        /// Show at most this many commits.
        #[arg(short = 'n', long, value_name = "N")]
        max_count: Option<usize>,
        /// One line per commit.
        #[arg(long)]
        oneline: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => {
            println!("mnem {}", env!("CARGO_PKG_VERSION"));
            println!("Version control for AI agent memory. Run `mnem --help`.");
            Ok(())
        }
        Some(Command::Init { path }) => cmd_init(path),
        Some(Command::Add {
            id,
            content,
            content_file,
            step,
            source,
            event_time,
        }) => cmd_add(id, content, content_file, step, source, event_time),
        Some(Command::Commit { message, author }) => cmd_commit(message, author),
        Some(Command::Log {
            first_parent,
            max_count,
            oneline,
        }) => cmd_log(first_parent, max_count, oneline),
    }
}

fn cmd_init(path: Option<PathBuf>) -> Result<()> {
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    let store = Store::init(&path)?;
    println!(
        "Initialised an empty Mnemosyne store in {}",
        store.mnem_dir().display()
    );
    Ok(())
}

fn cmd_add(
    id: String,
    content: Option<String>,
    content_file: Option<PathBuf>,
    step: Option<String>,
    source: Option<String>,
    event_time: Option<i64>,
) -> Result<()> {
    let content = match (content, content_file) {
        (_, Some(file)) => {
            let text =
                fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            serde_json::from_str(&text)
                .with_context(|| format!("{} is not valid JSON", file.display()))?
        }
        (Some(text), None) => serde_json::Value::String(text),
        (None, None) => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            serde_json::Value::String(buf.trim_end().to_string())
        }
    };

    let node = MemoryNode {
        id: id.clone(),
        content,
        content_kind: ContentKind::Note,
        provenance: Provenance {
            agent_step: step,
            source,
            ..Default::default()
        },
        event_time,
    };

    let store = open_store()?;
    let object_id = store.stage(&node)?;
    println!("Staged {id} as {}", short(&object_id));
    Ok(())
}

fn cmd_commit(message: String, author: Option<String>) -> Result<()> {
    let author = author
        .or_else(|| std::env::var("MNEM_AUTHOR").ok())
        .unwrap_or_else(|| "unknown".to_string());

    let store = open_store()?;
    if store.staged()?.is_empty() {
        anyhow::bail!("nothing staged; use `mnem add` first");
    }
    let id = store.commit(&message, &author, now_ms())?;
    println!(
        "[{}] {}",
        short(&id),
        message.lines().next().unwrap_or_default()
    );
    Ok(())
}

fn cmd_log(first_parent: bool, max_count: Option<usize>, oneline: bool) -> Result<()> {
    let store = open_store()?;
    let entries = store.log_head(first_parent, max_count)?;
    if entries.is_empty() {
        println!("no commits yet");
        return Ok(());
    }
    for (id, commit) in entries {
        if oneline {
            println!(
                "{} {}",
                short(&id),
                commit.message.lines().next().unwrap_or_default()
            );
        } else {
            println!("commit {id}");
            println!("Author: {}", commit.author);
            println!("Date:   {}", render_time(commit.time));
            println!();
            for line in commit.message.lines() {
                println!("    {line}");
            }
            println!();
        }
    }
    Ok(())
}

fn open_store() -> Result<Store> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    Ok(Store::open(&cwd)?)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn short(id: &ObjectId) -> String {
    id.to_hex()[..12].to_string()
}

/// Render a Unix-millisecond timestamp as `YYYY-MM-DD HH:MM:SSZ` (UTC).
///
/// Uses Howard Hinnant's civil-from-days algorithm, so there is no date
/// dependency to keep the CLI's minimum Rust version low.
fn render_time(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (sod / 3600, (sod % 3600) / 60, sod % 60);

    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!("{year:04}-{month:02}-{day:02} {hh:02}:{mm:02}:{ss:02}Z")
}

#[cfg(test)]
mod tests {
    use super::render_time;

    #[test]
    fn render_time_is_correct() {
        assert_eq!(render_time(0), "1970-01-01 00:00:00Z");
        assert_eq!(render_time(1_757_170_500_000), "2025-09-06 14:55:00Z");
        // a leap day
        assert_eq!(render_time(1_582_934_400_000), "2020-02-29 00:00:00Z");
    }
}
