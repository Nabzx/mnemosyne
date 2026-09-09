//! `mnem`, the Mnemosyne command line interface.
//!
//! Each subcommand is a thin wrapper over `mnemosyne-store` (ADR-0004). Phase 1:
//! `init`, `add`, `commit`, `log`. Phase 2 (ADR-0012): `branch`, `checkout`,
//! `show`, `diff`, `status`, `rm`. Phase 3 (ADR-0013, ADR-0014): `merge`.
//! Phase 4 (ADR-0015): `blame`, `bisect`, `show --stat`. See `ROADMAP.md`.
//!
//! Output is coloured when stdout is a terminal (see [`style`]); piped or
//! redirected output, and output with `NO_COLOR` set, stays plain.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use mnemosyne_store::{
    bisect, ChangeKind, Checkout, Conflict, ConflictKind, ContentKind, DiffTarget, Head,
    MemoryNode, MergeOutcome, MergeStrategy, NodeChange, ObjectId, Provenance, Resolution, Store,
};

mod style;

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
    /// Stage the deletion of a node from the next commit.
    Rm {
        /// The node id to remove.
        id: String,
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
    /// The current branch, HEAD, and what is staged.
    Status,
    /// List, create, or delete branches.
    Branch {
        /// The branch to create. Omit to list branches.
        name: Option<String>,
        /// Where the new branch starts: a branch name or commit id (default HEAD).
        start: Option<String>,
        /// Delete this branch.
        #[arg(short = 'd', long, value_name = "NAME", conflicts_with_all = ["name", "start"])]
        delete: Option<String>,
    },
    /// Move HEAD to a branch or commit.
    Checkout {
        /// A branch or commit to switch to. With -b, the new branch's start point.
        rev: Option<String>,
        /// Create a branch and switch to it.
        #[arg(short = 'b', long, value_name = "NAME")]
        create: Option<String>,
        /// Drop uncommitted staged changes instead of refusing.
        #[arg(long)]
        discard: bool,
    },
    /// Print the full memory at a commit, or working memory with no argument.
    Show {
        /// A branch name or commit id.
        rev: Option<String>,
        /// Instead of the memory, list the node ids this commit changed.
        #[arg(long)]
        stat: bool,
    },
    /// Resolve a memory node to the commit and provenance that introduced it.
    Blame {
        /// The node id to blame.
        node_id: String,
        /// Blame as of this branch or commit (default: HEAD).
        rev: Option<String>,
    },
    /// Find the first commit where a memory node reaches a given state.
    Bisect {
        /// The node id the predicate is about.
        #[arg(long)]
        node: String,
        /// True when the node's content equals this JSON value.
        #[arg(long, value_name = "JSON", conflicts_with_all = ["absent", "present"])]
        equals: Option<String>,
        /// True when the node is not in memory.
        #[arg(long, conflicts_with = "present")]
        absent: bool,
        /// True when the node is in memory.
        #[arg(long)]
        present: bool,
        /// A commit where the predicate is false (default: the history root).
        #[arg(long)]
        good: Option<String>,
        /// A commit where the predicate is true (default: HEAD).
        bad: Option<String>,
    },
    /// Merge another branch or commit into the current branch.
    Merge {
        /// A branch name or commit id to merge in.
        theirs: String,
        /// Resolve one conflict: `--resolve <id>=<ours|theirs|base|delete>`. Repeatable.
        #[arg(long, value_name = "ID=SIDE")]
        resolve: Vec<String>,
        /// Resolve every conflict one way: `ours` or `theirs`.
        #[arg(long, value_name = "SIDE")]
        strategy: Option<String>,
        /// Override the merge commit message.
        #[arg(short, long)]
        message: Option<String>,
        /// Who or what made the merge. Falls back to $MNEM_AUTHOR, then "unknown".
        #[arg(long)]
        author: Option<String>,
    },
    /// Show what changed between two memory states.
    Diff {
        /// The `from` side: a branch or commit. Default: the HEAD commit.
        from: Option<String>,
        /// The `to` side: a branch or commit. Default: working memory.
        to: Option<String>,
        /// One classified line per node, no content.
        #[arg(long)]
        stat: bool,
        /// Just the changed node ids.
        #[arg(long = "name-only")]
        name_only: bool,
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
        Some(Command::Rm { id }) => cmd_rm(id),
        Some(Command::Commit { message, author }) => cmd_commit(message, author),
        Some(Command::Log {
            first_parent,
            max_count,
            oneline,
        }) => cmd_log(first_parent, max_count, oneline),
        Some(Command::Status) => cmd_status(),
        Some(Command::Branch {
            name,
            start,
            delete,
        }) => cmd_branch(name, start, delete),
        Some(Command::Checkout {
            rev,
            create,
            discard,
        }) => cmd_checkout(rev, create, discard),
        Some(Command::Show { rev, stat }) => cmd_show(rev, stat),
        Some(Command::Blame { node_id, rev }) => cmd_blame(node_id, rev),
        Some(Command::Bisect {
            node,
            equals,
            absent,
            present,
            good,
            bad,
        }) => cmd_bisect(node, equals, absent, present, good, bad),
        Some(Command::Merge {
            theirs,
            resolve,
            strategy,
            message,
            author,
        }) => cmd_merge(theirs, resolve, strategy, message, author),
        Some(Command::Diff {
            from,
            to,
            stat,
            name_only,
        }) => cmd_diff(from, to, stat, name_only),
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
    println!(
        "Staged {} as {}",
        style::id(&id),
        style::dim(&short(&object_id))
    );
    Ok(())
}

fn cmd_rm(id: String) -> Result<()> {
    let store = open_store()?;
    store.rm(&id)?;
    println!("Staged deletion of {}", style::id(&id));
    Ok(())
}

fn cmd_commit(message: String, author: Option<String>) -> Result<()> {
    let author = author
        .or_else(|| std::env::var("MNEM_AUTHOR").ok())
        .unwrap_or_else(|| "unknown".to_string());

    let store = open_store()?;
    if store.staged()?.is_empty() && store.staged_deletions()?.is_empty() {
        anyhow::bail!("nothing staged; use `mnem add` or `mnem rm` first");
    }
    let id = store.commit(&message, &author, now_ms())?;
    println!(
        "{} {}",
        style::good(&format!("[{}]", short(&id))),
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
                style::id(&short(&id)),
                commit.message.lines().next().unwrap_or_default()
            );
        } else {
            println!("commit {}", style::id(&id.to_hex()));
            println!("Author: {}", style::dim(&commit.author));
            println!("Date:   {}", style::dim(&render_time(commit.time)));
            println!();
            for line in commit.message.lines() {
                println!("    {line}");
            }
            println!();
        }
    }
    Ok(())
}

fn cmd_status() -> Result<()> {
    let store = open_store()?;
    match store.head()? {
        Head::Attached(name) => println!("On branch {}", style::branch(&name)),
        Head::Detached(id) => println!("HEAD detached at {}", style::id(&short(&id))),
    }

    match store.head_commit()? {
        Some(tip) => {
            let msg = tip_message(&store, tip)?;
            println!("  {}  {msg}", style::id(&short(&tip)));
        }
        None => println!("  {}", style::dim("(no commits yet)")),
    }

    let from = head_diff_target(&store)?;
    let changes = store.diff(from, DiffTarget::Working)?;
    if changes.is_empty() {
        println!("nothing staged");
    } else {
        println!("Staged:");
        for change in changes {
            println!(
                "  {}",
                paint_change(&format!("{} {}", marker(&change), change.id()), &change)
            );
        }
    }
    Ok(())
}

fn cmd_branch(name: Option<String>, start: Option<String>, delete: Option<String>) -> Result<()> {
    let store = open_store()?;

    if let Some(name) = delete {
        if store.delete_branch(&name)? {
            println!("Deleted branch {}", style::branch(&name));
            return Ok(());
        }
        anyhow::bail!("no branch '{name}'");
    }

    match name {
        Some(name) => {
            store.branch(&name, start.as_deref())?;
            let at = store.resolve_commitish(&name)?;
            println!(
                "Created branch {} at {}",
                style::branch(&name),
                style::id(&short(&at))
            );
        }
        None => {
            let current = match store.head()? {
                Head::Attached(n) => Some(n),
                Head::Detached(_) => None,
            };
            let branches = store.branches()?;
            if branches.is_empty() {
                println!("no branches yet");
                return Ok(());
            }
            for (name, tip) in branches {
                let here = Some(&name) == current.as_ref();
                let flag = if here {
                    style::good("*")
                } else {
                    " ".to_string()
                };
                let shown = if here {
                    style::good(&name)
                } else {
                    style::branch(&name)
                };
                let msg = tip_message(&store, tip)?;
                println!("{flag} {shown}  {}  {msg}", style::id(&short(&tip)));
            }
        }
    }
    Ok(())
}

fn cmd_checkout(rev: Option<String>, create: Option<String>, discard: bool) -> Result<()> {
    let store = open_store()?;

    let target = match create {
        Some(name) => {
            store.branch(&name, rev.as_deref())?;
            name
        }
        None => rev.context("checkout needs a target, or -b <name> to create one")?,
    };

    match store.checkout(&target, discard)? {
        Checkout::SwitchedToBranch(name) => {
            println!("Switched to branch {}", style::branch(&name));
        }
        Checkout::DetachedAt(id) => {
            println!("HEAD is now at {} (detached)", style::id(&short(&id)));
        }
        Checkout::AlreadyThere => println!("Already on {}", style::branch(&target)),
    }
    Ok(())
}

fn cmd_show(rev: Option<String>, stat: bool) -> Result<()> {
    let store = open_store()?;

    if stat {
        let commit = match rev {
            Some(spec) => store.resolve_commitish(&spec)?,
            None => store
                .head_commit()?
                .context("nothing to show: HEAD has no commit")?,
        };
        let changed = store.changed_by(commit)?;
        if changed.is_empty() {
            println!("no changes");
            return Ok(());
        }
        for (id, kind) in changed {
            let line = match kind {
                ChangeKind::Added => style::good(&format!("+ {id}")),
                ChangeKind::Modified => style::changed(&format!("~ {id}")),
                ChangeKind::Removed => style::bad(&format!("- {id}")),
            };
            println!("{line}");
        }
        return Ok(());
    }

    let memory = match rev {
        Some(spec) => store.state_at(store.resolve_commitish(&spec)?)?,
        None => store.working_memory()?,
    };
    if memory.is_empty() {
        println!("{}", style::dim("(empty)"));
        return Ok(());
    }
    for (id, node) in memory {
        println!("{}", style::id(&id));
        println!("  {}", serde_json::to_string(&node.content)?);
    }
    Ok(())
}

fn cmd_blame(node_id: String, rev: Option<String>) -> Result<()> {
    let store = open_store()?;
    let blame = store.blame(&node_id, rev.as_deref())?;

    println!(
        "{}  {}",
        style::id(&short(&blame.commit)),
        style::dim(&render_time(blame.time))
    );
    println!(
        "  {} {}",
        style::dim("commit: "),
        tip_message(&store, blame.commit)?
    );
    println!(
        "  {} {}",
        style::dim("content:"),
        serde_json::to_string(&blame.node.content)?
    );

    let p = &blame.node.provenance;
    let fields = [
        ("step", &p.agent_step),
        ("observation", &p.observation),
        ("tool_call", &p.tool_call),
        ("source", &p.source),
        ("note", &p.note),
    ];
    let mut any = false;
    for (label, value) in fields {
        if let Some(v) = value {
            println!("  {} {}", style::dim(&format!("{label}:")), style::warn(v));
            any = true;
        }
    }
    if !any {
        println!("  {}", style::dim("(no provenance recorded)"));
    }
    Ok(())
}

fn cmd_bisect(
    node: String,
    equals: Option<String>,
    absent: bool,
    present: bool,
    good: Option<String>,
    bad: Option<String>,
) -> Result<()> {
    let store = open_store()?;

    let boundary = if let Some(json) = &equals {
        let value: serde_json::Value = serde_json::from_str(json)
            .with_context(|| format!("--equals {json:?} is not valid JSON"))?;
        store.bisect(
            bad.as_deref(),
            good.as_deref(),
            bisect::node_content_is(node.clone(), value),
        )?
    } else if absent {
        store.bisect(
            bad.as_deref(),
            good.as_deref(),
            bisect::node_absent(node.clone()),
        )?
    } else if present {
        store.bisect(
            bad.as_deref(),
            good.as_deref(),
            bisect::node_present(node.clone()),
        )?
    } else {
        anyhow::bail!("bisect needs one of --equals <json>, --absent or --present");
    };

    println!(
        "{}  {}",
        style::id(&short(&boundary)),
        tip_message(&store, boundary)?
    );

    // explain the boundary: blame the node there
    match store.blame(&node, Some(&boundary.to_hex())) {
        Ok(blame) => {
            let p = &blame.node.provenance;
            let mut bits = Vec::new();
            if let Some(s) = &p.agent_step {
                bits.push(format!("step {s}"));
            }
            if let Some(o) = &p.observation {
                bits.push(format!("observation {o}"));
            }
            if let Some(s) = &p.source {
                bits.push(format!("source {s}"));
            }
            let here = style::warn(&format!("{node} set here"));
            if bits.is_empty() {
                println!("  {here}{}", style::dim(", no provenance recorded"));
            } else {
                println!(
                    "  {here}{}",
                    style::dim(&format!(", from {}", bits.join(", ")))
                );
            }
        }
        Err(_) => println!("  {}", style::dim(&format!("{node} is absent here"))),
    }
    Ok(())
}

fn cmd_merge(
    theirs: String,
    resolve: Vec<String>,
    strategy: Option<String>,
    message: Option<String>,
    author: Option<String>,
) -> Result<()> {
    let author = author
        .or_else(|| std::env::var("MNEM_AUTHOR").ok())
        .unwrap_or_else(|| "unknown".to_string());

    let strategy = match strategy.as_deref() {
        None => None,
        Some("ours") => Some(MergeStrategy::Ours),
        Some("theirs") => Some(MergeStrategy::Theirs),
        Some(other) => anyhow::bail!("unknown --strategy {other:?}; use `ours` or `theirs`"),
    };

    let mut resolutions: BTreeMap<String, Resolution> = BTreeMap::new();
    for spec in &resolve {
        let (id, side) = spec.split_once('=').with_context(|| {
            format!("--resolve {spec:?} must be <id>=<ours|theirs|base|delete>")
        })?;
        let resolution = match side {
            "ours" => Resolution::Ours,
            "theirs" => Resolution::Theirs,
            "base" => Resolution::Base,
            "delete" => Resolution::Delete,
            other => anyhow::bail!(
                "--resolve {spec:?}: unknown side {other:?}; use `ours`, `theirs`, `base` or `delete`"
            ),
        };
        resolutions.insert(id.to_string(), resolution);
    }

    let store = open_store()?;
    let branch = match store.head()? {
        Head::Attached(name) => name,
        Head::Detached(_) => anyhow::bail!("cannot merge from a detached HEAD; check out a branch"),
    };

    match store.merge(
        &theirs,
        &resolutions,
        strategy,
        message.as_deref(),
        &author,
        now_ms(),
    )? {
        MergeOutcome::AlreadyUpToDate => println!("Already up to date."),
        MergeOutcome::FastForwarded(id) => {
            println!(
                "Fast-forwarded {} to {}",
                style::branch(&branch),
                style::id(&short(&id))
            );
        }
        MergeOutcome::Merged(id) => {
            let msg = tip_message(&store, id)?;
            println!("{} {msg}", style::good(&format!("[{}]", short(&id))));
        }
        MergeOutcome::Conflicts(conflicts) => {
            println!(
                "{}",
                style::warn(&format!("Merge conflict in {} node(s):", conflicts.len()))
            );
            for conflict in &conflicts {
                print_conflict(&store, conflict)?;
            }
            anyhow::bail!(
                "resolve with `--resolve <id>=<side>` (or `--strategy ours|theirs`) and merge again"
            );
        }
    }
    Ok(())
}

fn print_conflict(store: &Store, conflict: &Conflict) -> Result<()> {
    println!(
        "  {} {}",
        style::id(&conflict.id),
        style::dim(&format!("({})", conflict_kind_label(conflict.kind)))
    );
    let show = |label: &str, side: Option<ObjectId>| -> Result<()> {
        let label = style::dim(&format!("{label:<7}"));
        match side {
            Some(object_id) => println!("    {label} {}", node_content(store, object_id)?),
            None => println!("    {label} (deleted)"),
        }
        Ok(())
    };
    show("base:", conflict.base)?;
    show("ours:", conflict.ours)?;
    show("theirs:", conflict.theirs)?;
    Ok(())
}

fn conflict_kind_label(kind: ConflictKind) -> &'static str {
    match kind {
        ConflictKind::EditEdit => "edit/edit",
        ConflictKind::DeleteEdit => "delete/edit",
        ConflictKind::EditDelete => "edit/delete",
        ConflictKind::AddAdd => "add/add",
    }
}

fn cmd_diff(from: Option<String>, to: Option<String>, stat: bool, name_only: bool) -> Result<()> {
    let store = open_store()?;

    let (from_target, to_target) = match (from, to) {
        (None, _) => (head_diff_target(&store)?, DiffTarget::Working),
        (Some(a), None) => (
            DiffTarget::Commit(store.resolve_commitish(&a)?),
            DiffTarget::Working,
        ),
        (Some(a), Some(b)) => (
            DiffTarget::Commit(store.resolve_commitish(&a)?),
            DiffTarget::Commit(store.resolve_commitish(&b)?),
        ),
    };

    let changes = store.diff(from_target, to_target)?;
    if changes.is_empty() {
        println!("no changes");
        return Ok(());
    }

    for change in &changes {
        if name_only {
            println!("{}", change.id());
            continue;
        }
        println!(
            "{}",
            paint_change(&format!("{} {}", marker(change), change.id()), change)
        );
        if stat {
            continue;
        }
        match change {
            NodeChange::Added { new, .. } => {
                println!(
                    "{}",
                    style::good(&format!("  + {}", node_content(&store, *new)?))
                );
            }
            NodeChange::Removed { old, .. } => {
                println!(
                    "{}",
                    style::bad(&format!("  - {}", node_content(&store, *old)?))
                );
            }
            NodeChange::Modified { old, new, .. } => {
                println!(
                    "{}",
                    style::bad(&format!("  - {}", node_content(&store, *old)?))
                );
                println!(
                    "{}",
                    style::good(&format!("  + {}", node_content(&store, *new)?))
                );
            }
        }
    }
    Ok(())
}

fn open_store() -> Result<Store> {
    let cwd = std::env::current_dir().context("reading the current directory")?;
    Ok(Store::open(&cwd)?)
}

fn head_diff_target(store: &Store) -> Result<DiffTarget> {
    Ok(match store.head_commit()? {
        Some(id) => DiffTarget::Commit(id),
        None => DiffTarget::Empty,
    })
}

fn tip_message(store: &Store, tip: ObjectId) -> Result<String> {
    let message = store
        .log(tip, true, Some(1))?
        .into_iter()
        .next()
        .map(|(_, commit)| commit.message)
        .unwrap_or_default();
    Ok(message.lines().next().unwrap_or_default().to_string())
}

fn node_content(store: &Store, object_id: ObjectId) -> Result<String> {
    Ok(serde_json::to_string(&store.node(object_id)?.content)?)
}

fn marker(change: &NodeChange) -> char {
    match change {
        NodeChange::Added { .. } => '+',
        NodeChange::Removed { .. } => '-',
        NodeChange::Modified { .. } => '~',
    }
}

/// Colour `line` by the kind of change it represents.
fn paint_change(line: &str, change: &NodeChange) -> String {
    match change {
        NodeChange::Added { .. } => style::good(line),
        NodeChange::Removed { .. } => style::bad(line),
        NodeChange::Modified { .. } => style::changed(line),
    }
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
