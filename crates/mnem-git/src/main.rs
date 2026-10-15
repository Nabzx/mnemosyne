//! `mnem`, the Mnemosyne command line interface: version control for AI agent
//! memory.
//!
//! `init` a store, `add` and `commit` memory with its provenance, `branch` and
//! `checkout` to explore a hypothesis, `merge` two lines of memory back
//! together, and `blame` or `bisect` to trace a wrong belief back to the
//! observation that caused it. Every subcommand is a thin wrapper over
//! `mnem-store` (ADR-0004); this crate holds no operation semantics of its
//! own, only argument parsing, formatting and the exit codes.
//!
//! Output is coloured when stdout is a terminal (see [`style`]); piped or
//! redirected output, and output with `NO_COLOR` set, stays plain.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use mnem_store::{
    bisect, ChangeKind, Checkout, Conflict, ConflictKind, ContentKind, DiffTarget, Export, Head,
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
        /// The content, parsed as JSON (a number, bool, object, array, or
        /// null) instead of stored as a plain string.
        #[arg(long, value_name = "JSON", conflicts_with_all = ["content", "content_file"])]
        content_json: Option<String>,
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
    Status {
        /// A dashboard view: every branch, its tip's author, and how long
        /// ago it last moved - instead of just the current branch.
        #[arg(long = "all-branches")]
        all_branches: bool,
    },
    /// List every branch not yet merged into `main`, with its tip commit.
    Inbox,
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
        /// Show what merging would do, without writing anything. Not combined
        /// with `--resolve` or `--strategy`.
        #[arg(long = "dry-run")]
        dry_run: bool,
        /// With `--dry-run`: if there are conflicts, also render them as a
        /// static HTML page (base/ours/theirs side by side) at this path.
        #[arg(long, value_name = "FILE", requires = "dry_run")]
        html: Option<PathBuf>,
    },
    /// Approve a branch: merge it in with the reviewer recorded as the
    /// merge commit's author. A thin wrapper over `merge`, not the
    /// ADR-gated propose/approve object #419-422 will eventually settle.
    Approve {
        /// The branch to approve and merge in.
        branch: String,
        /// Who is approving. Recorded as the merge commit's author.
        #[arg(long)]
        reviewer: String,
    },
    /// Reject a branch: record why, without touching the store.
    Reject {
        /// The branch being rejected.
        branch: String,
        /// Who is rejecting.
        #[arg(long)]
        reviewer: String,
        /// Why the branch is rejected.
        #[arg(long)]
        reason: String,
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
    /// Narrate what changed between two memory states in plain English.
    Changelog {
        /// The `from` side: a branch or commit. Default: the HEAD commit.
        from: Option<String>,
        /// The `to` side: a branch or commit. Default: working memory.
        to: Option<String>,
    },
    /// Flag whether two nodes in working memory might be the same claim,
    /// phrased differently - a crude word-overlap heuristic, not the real
    /// semantic-merge detector.
    Similar {
        /// The first node id.
        a: String,
        /// The second node id.
        b: String,
    },
    /// Flag whether two nodes in working memory might be direct negations
    /// of each other - shared vocabulary plus a negation word on only one
    /// side. A crude heuristic, not the real contradiction detector #234
    /// and #423 already deferred past ADR-0018.
    Contradicts {
        /// The first node id.
        a: String,
        /// The second node id.
        b: String,
    },
    /// Print a shell completion script to stdout.
    Completions {
        /// Which shell to generate for.
        shell: Shell,
    },
    /// Dump the whole store (every commit, ref and HEAD) as one JSON value.
    Export {
        /// Write here instead of stdout.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
    /// Create a new store from an export (the reverse of `export`).
    Import {
        /// The exported JSON file.
        file: PathBuf,
        /// Where to create the store. Refuses if one already exists there.
        path: Option<PathBuf>,
    },
    /// A local file-based registry: a folder of exported agent bundles,
    /// one JSON file each. #451: a throwaway spike on the file-based side
    /// of the hosted-vs-file-based decision #407, built entirely from
    /// `export`/`import` - no new object kind, no network.
    Registry {
        #[command(subcommand)]
        action: RegistryAction,
    },
}

#[derive(Subcommand)]
enum RegistryAction {
    /// Export the current store and publish it to the registry as `<name>.json`.
    Publish {
        /// The bundle's name in the registry.
        name: String,
        /// The registry folder. Falls back to $MNEM_REGISTRY.
        #[arg(long, value_name = "DIR")]
        registry: Option<PathBuf>,
    },
    /// List every bundle in the registry.
    List {
        /// The registry folder. Falls back to $MNEM_REGISTRY.
        #[arg(long, value_name = "DIR")]
        registry: Option<PathBuf>,
    },
    /// Fork a published bundle into a new store (import under a new name).
    Fork {
        /// The bundle's name in the registry.
        name: String,
        /// Where to create the forked store. Refuses if one already exists there.
        dest: PathBuf,
        /// The registry folder. Falls back to $MNEM_REGISTRY.
        #[arg(long, value_name = "DIR")]
        registry: Option<PathBuf>,
    },
    /// Render every bundle in the registry as one static HTML page:
    /// name, prompt summary, and tool count per bundle (#453). Reads
    /// `agent:prompt`/`agent:tools` - #450's own demo convention, not a
    /// real one - so a bundle missing either just shows "(none)".
    Render {
        /// The registry folder. Falls back to $MNEM_REGISTRY.
        #[arg(long, value_name = "DIR")]
        registry: Option<PathBuf>,
        /// Write the page here.
        #[arg(long, value_name = "FILE")]
        output: PathBuf,
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
            content_json,
            step,
            source,
            event_time,
        }) => cmd_add(
            id,
            content,
            content_file,
            content_json,
            step,
            source,
            event_time,
        ),
        Some(Command::Rm { id }) => cmd_rm(id),
        Some(Command::Commit { message, author }) => cmd_commit(message, author),
        Some(Command::Log {
            first_parent,
            max_count,
            oneline,
        }) => cmd_log(first_parent, max_count, oneline),
        Some(Command::Status { all_branches }) => cmd_status(all_branches),
        Some(Command::Inbox) => cmd_inbox(),
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
            dry_run,
            html,
        }) => cmd_merge(theirs, resolve, strategy, message, author, dry_run, html),
        Some(Command::Approve { branch, reviewer }) => cmd_approve(branch, reviewer),
        Some(Command::Reject {
            branch,
            reviewer,
            reason,
        }) => cmd_reject(branch, reviewer, reason),
        Some(Command::Diff {
            from,
            to,
            stat,
            name_only,
        }) => cmd_diff(from, to, stat, name_only),
        Some(Command::Changelog { from, to }) => cmd_changelog(from, to),
        Some(Command::Similar { a, b }) => cmd_similar(a, b),
        Some(Command::Contradicts { a, b }) => cmd_contradicts(a, b),
        Some(Command::Completions { shell }) => cmd_completions(shell),
        Some(Command::Export { output }) => cmd_export(output),
        Some(Command::Import { file, path }) => cmd_import(file, path),
        Some(Command::Registry { action }) => match action {
            RegistryAction::Publish { name, registry } => cmd_registry_publish(name, registry),
            RegistryAction::List { registry } => cmd_registry_list(registry),
            RegistryAction::Fork {
                name,
                dest,
                registry,
            } => cmd_registry_fork(name, dest, registry),
            RegistryAction::Render { registry, output } => cmd_registry_render(registry, output),
        },
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
    content_json: Option<String>,
    step: Option<String>,
    source: Option<String>,
    event_time: Option<i64>,
) -> Result<()> {
    let content = match (content, content_file, content_json) {
        (_, Some(file), _) => {
            let text =
                fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            serde_json::from_str(&text)
                .with_context(|| format!("{} is not valid JSON", file.display()))?
        }
        (_, None, Some(json)) => {
            serde_json::from_str(&json).context("--content-json is not valid JSON")?
        }
        (Some(text), None, None) => serde_json::Value::String(text),
        (None, None, None) => {
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

fn cmd_status(all_branches: bool) -> Result<()> {
    let store = open_store()?;
    if all_branches {
        return cmd_status_all_branches(&store);
    }
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

/// `mnem status --all-branches`: every branch, its tip's real author, and
/// how long ago it last moved - a dashboard-shaped read over data that
/// already exists. A market-test spike (#446); a human judges whether this
/// earns its keep over just reading `mnem branch`.
fn cmd_status_all_branches(store: &Store) -> Result<()> {
    let branches = store.branches()?;
    if branches.is_empty() {
        println!("no branches yet");
        return Ok(());
    }
    let now = now_ms();
    for (name, tip) in branches {
        let commit = store.log(tip, true, Some(1))?.into_iter().next();
        match commit {
            Some((_, commit)) => {
                println!(
                    "{}  {}  {}  {}",
                    style::branch(&name),
                    style::id(&short(&tip)),
                    style::dim(&commit.author),
                    style::dim(&humanize_age(now - commit.time))
                );
            }
            None => println!(
                "{}  {}  {}",
                style::branch(&name),
                style::id(&short(&tip)),
                style::dim("(no commits)")
            ),
        }
    }
    Ok(())
}

/// A commit age in milliseconds as a short, human-readable string:
/// "5s ago", "12m ago", "3h ago", "2d ago".
fn humanize_age(age_ms: i64) -> String {
    let secs = age_ms.max(0) / 1000;
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86_400)
    }
}

/// List every branch not yet merged into `main`, with its tip commit's
/// author and message - a market-test spike (#441), not the ADR-gated
/// propose/approve object #419-422 will eventually settle. A read-only view
/// over branches that already exist; no new object model.
fn cmd_inbox() -> Result<()> {
    let store = open_store()?;
    let branches = store.branches()?;
    let main_tip = branches
        .iter()
        .find(|(name, _)| name == "main")
        .map(|(_, tip)| *tip);

    let mut pending = Vec::new();
    for (name, tip) in &branches {
        if name == "main" {
            continue;
        }
        let already_merged = match main_tip {
            Some(main_tip) => *tip == main_tip || store.is_ancestor(*tip, main_tip)?,
            None => false,
        };
        if !already_merged {
            pending.push((name.clone(), *tip));
        }
    }

    if pending.is_empty() {
        println!("Nothing pending - every branch is merged into main.");
        return Ok(());
    }

    for (name, tip) in pending {
        let commit = store.log(tip, true, Some(1))?.into_iter().next();
        match commit {
            Some((_, commit)) => {
                let first_line = commit.message.lines().next().unwrap_or_default();
                println!(
                    "{}  {}  {}  {first_line}",
                    style::branch(&name),
                    style::id(&short(&tip)),
                    style::dim(&commit.author),
                );
            }
            None => println!(
                "{}  {}  {}",
                style::branch(&name),
                style::id(&short(&tip)),
                style::dim("(no commits)")
            ),
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
    dry_run: bool,
    html: Option<PathBuf>,
) -> Result<()> {
    if dry_run {
        if !resolve.is_empty() || strategy.is_some() {
            anyhow::bail!("--dry-run previews whether a merge collides; it does not combine with --resolve or --strategy");
        }
        return cmd_merge_dry_run(theirs, html);
    }

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

/// Preview `merge`'s own decision (clean, fast-forward, or conflicts) without
/// writing anything - computed straight from the same public primitives
/// `Store::merge` itself uses (`merge_base`, `merge_states`), not by running a
/// real merge and rolling it back. #440: a market-test spike, not a change to
/// the merge algorithm or its public API.
fn cmd_merge_dry_run(theirs: String, html: Option<PathBuf>) -> Result<()> {
    let store = open_store()?;
    let branch = match store.head()? {
        Head::Attached(name) => name,
        Head::Detached(_) => anyhow::bail!("cannot merge from a detached HEAD; check out a branch"),
    };

    let theirs_tip = store.resolve_commitish(&theirs)?;
    let Some(ours_tip) = store.head_commit()? else {
        println!(
            "(dry run) {} has no commits yet; would fast-forward to {}",
            style::branch(&branch),
            style::id(&short(&theirs_tip))
        );
        return Ok(());
    };

    if theirs_tip == ours_tip || store.is_ancestor(theirs_tip, ours_tip)? {
        println!("Already up to date.");
        return Ok(());
    }
    if store.is_ancestor(ours_tip, theirs_tip)? {
        println!(
            "(dry run) would fast-forward {} to {}",
            style::branch(&branch),
            style::id(&short(&theirs_tip))
        );
        return Ok(());
    }

    let base = store.merge_base(ours_tip, theirs_tip)?;
    let state_merge = store.merge_states(base, ours_tip, theirs_tip)?;
    if state_merge.is_clean() {
        println!("(dry run) would merge cleanly, no conflicts.");
        return Ok(());
    }

    println!(
        "{}",
        style::warn(&format!(
            "(dry run) would conflict in {} node(s):",
            state_merge.conflicts.len()
        ))
    );
    for conflict in &state_merge.conflicts {
        print_conflict(&store, conflict)?;
    }
    if let Some(path) = html {
        let page = render_conflicts_html(&store, &branch, &theirs, &state_merge.conflicts)?;
        fs::write(&path, page)
            .with_context(|| format!("writing the conflict page to {}", path.display()))?;
        println!(
            "Wrote {} to {}",
            style::good("the conflict page"),
            path.display()
        );
    }
    anyhow::bail!(
        "resolve with `--resolve <id>=<side>` (or `--strategy ours|theirs`) and merge for real"
    )
}

/// Render `conflicts` as one static HTML page, base/ours/theirs side by
/// side per conflicting node - a screenshot-shaped view over the same
/// data `print_conflict` already prints as text, not a new computation.
/// No server, no interactivity, no new dependency: content is escaped by
/// hand (`escape_html`) rather than pulling in an HTML-templating crate
/// for a throwaway spike (#448).
fn render_conflicts_html(
    store: &Store,
    ours: &str,
    theirs: &str,
    conflicts: &[Conflict],
) -> Result<String> {
    let mut rows = String::new();
    for conflict in conflicts {
        let cell = |side: Option<ObjectId>| -> Result<String> {
            Ok(match side {
                Some(id) => format!("<pre>{}</pre>", escape_html(&node_content(store, id)?)),
                None => "<pre class=\"deleted\">(deleted)</pre>".to_string(),
            })
        };
        rows.push_str(&format!(
            "<tr>\n  <th>{}<span class=\"kind\">{}</span></th>\n  <td>{}</td>\n  <td>{}</td>\n  <td>{}</td>\n</tr>\n",
            escape_html(&conflict.id),
            conflict_kind_label(conflict.kind),
            cell(conflict.base)?,
            cell(conflict.ours)?,
            cell(conflict.theirs)?,
        ));
    }
    Ok(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>mnem merge conflicts: {ours} vs {theirs}</title>
<style>
body {{ font-family: -apple-system, sans-serif; background: #1e1e1e; color: #ddd; margin: 2rem; }}
h1 {{ font-size: 1.1rem; font-weight: 600; }}
table {{ border-collapse: collapse; width: 100%; margin-top: 1rem; }}
th, td {{ border: 1px solid #444; padding: 0.5rem; vertical-align: top; text-align: left; width: 25%; }}
th {{ background: #272727; font-family: monospace; font-size: 0.85rem; }}
th .kind {{ display: block; color: #999; font-size: 0.75rem; }}
td:nth-child(2) {{ background: #22281f; }}
td:nth-child(3) {{ background: #2a1f1f; }}
td:nth-child(4) {{ background: #1f232a; }}
pre {{ margin: 0; white-space: pre-wrap; word-break: break-word; font-family: monospace; }}
.deleted {{ color: #888; font-style: italic; }}
</style>
</head>
<body>
<h1>Merge conflicts: <code>{ours}</code> vs <code>{theirs}</code></h1>
<table>
<tr><th>node</th><th>base</th><th>ours</th><th>theirs</th></tr>
{rows}</table>
</body>
</html>
"#,
        ours = escape_html(ours),
        theirs = escape_html(theirs),
        rows = rows,
    ))
}

/// Escape the five characters HTML cares about. No templating crate for
/// one throwaway spike - this is the whole of what `render_conflicts_html`
/// needs.
fn escape_html(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            other => other.to_string(),
        })
        .collect()
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

/// Approve a branch: the real `merge`, with the reviewer named as the
/// merge commit's author. #447: a market-test spike for the review-model
/// map (#418), not the ADR-gated propose/approve object #419-422 will
/// eventually settle. No resolve or strategy options of its own - a
/// conflicting merge still fails exactly as `merge` itself would, with the
/// same "resolve and merge again" message.
fn cmd_approve(branch: String, reviewer: String) -> Result<()> {
    cmd_merge(branch, Vec::new(), None, None, Some(reviewer), false, None)
}

/// Reject a branch: print who rejected it and why, and stop there. The
/// store is never opened, so nothing about it is touched.
fn cmd_reject(branch: String, reviewer: String, reason: String) -> Result<()> {
    println!(
        "{} {} ({}): {reason}",
        style::bad("Rejected"),
        style::branch(&branch),
        style::dim(&reviewer)
    );
    Ok(())
}

/// Resolve `diff`/`changelog`'s shared `from`/`to` arguments: no `from` means
/// HEAD compared against working memory; a lone `from` compares it to
/// working memory too; both compares two commits directly.
fn resolve_diff_range(
    store: &Store,
    from: Option<String>,
    to: Option<String>,
) -> Result<(DiffTarget, DiffTarget)> {
    Ok(match (from, to) {
        (None, _) => (head_diff_target(store)?, DiffTarget::Working),
        (Some(a), None) => (
            DiffTarget::Commit(store.resolve_commitish(&a)?),
            DiffTarget::Working,
        ),
        (Some(a), Some(b)) => (
            DiffTarget::Commit(store.resolve_commitish(&a)?),
            DiffTarget::Commit(store.resolve_commitish(&b)?),
        ),
    })
}

fn cmd_diff(from: Option<String>, to: Option<String>, stat: bool, name_only: bool) -> Result<()> {
    let store = open_store()?;
    let (from_target, to_target) = resolve_diff_range(&store, from, to)?;

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

/// Narrate `diff`'s own output in plain English, one sentence per node,
/// instead of a raw before/after dump - the same data `diff` already
/// computes, just reworded. A market-test spike (#442); `diff`'s own
/// structured output is unchanged.
fn cmd_changelog(from: Option<String>, to: Option<String>) -> Result<()> {
    let store = open_store()?;
    let (from_target, to_target) = resolve_diff_range(&store, from, to)?;

    let changes = store.diff(from_target, to_target)?;
    if changes.is_empty() {
        println!("no changes");
        return Ok(());
    }

    for change in &changes {
        let id = change.id();
        match change {
            NodeChange::Added { new, .. } => {
                let node = store.node(*new)?;
                println!(
                    "{id} was added, now {}{}",
                    node_content(&store, *new)?,
                    source_suffix(&node)
                );
            }
            NodeChange::Removed { old, .. } => {
                println!("{id} was removed, was {}", node_content(&store, *old)?);
            }
            NodeChange::Modified { old, new, .. } => {
                let node = store.node(*new)?;
                println!(
                    "{id} changed from {} to {}{}",
                    node_content(&store, *old)?,
                    node_content(&store, *new)?,
                    source_suffix(&node)
                );
            }
        }
    }
    Ok(())
}

/// `, sourced from <source>` when the node's own provenance names one;
/// empty otherwise.
fn source_suffix(node: &MemoryNode) -> String {
    match &node.provenance.source {
        Some(source) => format!(", sourced from {source}"),
        None => String::new(),
    }
}

/// A word-overlap heuristic flags two facts as possibly the same claim,
/// phrased differently - a crude, explicit spike (#443), not the real
/// embedding-based detector the semantic-merge map (#423) and the
/// contradiction map (#234) will eventually settle on. Never merges or
/// blocks anything; purely a suggestion to act on.
const SIMILARITY_THRESHOLD: f64 = 0.3;

fn cmd_similar(a: String, b: String) -> Result<()> {
    let store = open_store()?;
    let node_a = store
        .working_node(&a)?
        .with_context(|| format!("no node {a:?} in working memory"))?;
    let node_b = store
        .working_node(&b)?
        .with_context(|| format!("no node {b:?} in working memory"))?;

    println!(
        "{}: {}",
        style::id(&a),
        serde_json::to_string(&node_a.content)?
    );
    println!(
        "{}: {}",
        style::id(&b),
        serde_json::to_string(&node_b.content)?
    );

    let score = word_overlap(&node_a.content, &node_b.content);
    println!();
    if score >= SIMILARITY_THRESHOLD {
        println!(
            "{} might be the same claim, phrased differently (word overlap: {:.0}%)",
            style::warn("flagged:"),
            score * 100.0
        );
    } else {
        println!("not flagged (word overlap: {:.0}%)", score * 100.0);
    }
    Ok(())
}

/// Flag whether `a` and `b` look like direct negations of each other:
/// shared vocabulary above the same threshold `similar` uses, plus a
/// negation word on exactly one side, not both or neither - a fact and
/// its own double negative are not what this is looking for. #449: a
/// crude heuristic, not the real contradiction detector #234 and #423
/// already deferred past ADR-0018. Never blocks a commit, only suggests.
fn cmd_contradicts(a: String, b: String) -> Result<()> {
    let store = open_store()?;
    let node_a = store
        .working_node(&a)?
        .with_context(|| format!("no node {a:?} in working memory"))?;
    let node_b = store
        .working_node(&b)?
        .with_context(|| format!("no node {b:?} in working memory"))?;

    println!(
        "{}: {}",
        style::id(&a),
        serde_json::to_string(&node_a.content)?
    );
    println!(
        "{}: {}",
        style::id(&b),
        serde_json::to_string(&node_b.content)?
    );

    let overlap = word_overlap(&node_a.content, &node_b.content);
    let negates_a = has_negation(&node_a.content);
    let negates_b = has_negation(&node_b.content);

    println!();
    if overlap >= SIMILARITY_THRESHOLD && negates_a != negates_b {
        let negated_side = if negates_a { &a } else { &b };
        println!(
            "{} might be direct negations of each other (word overlap: {:.0}%, {} carries a negation word)",
            style::warn("flagged:"),
            overlap * 100.0,
            style::id(negated_side)
        );
    } else {
        println!(
            "not flagged (word overlap: {:.0}%, negation on {})",
            overlap * 100.0,
            match (negates_a, negates_b) {
                (true, true) => "both sides",
                (false, false) => "neither side",
                (true, false) => "only a, below the similarity threshold",
                (false, true) => "only b, below the similarity threshold",
            }
        );
    }
    Ok(())
}

/// Whether `content` carries at least one bare negation word - "not",
/// "never", "no", "none", "nothing", "nobody", "nowhere", "cannot" - as
/// its own token. Crude on purpose, and crude the same way
/// `significant_words` already is: splitting on non-alphanumeric
/// characters means a contraction like "doesn't" never matches (it
/// tokenises to "doesn" and "t"), so this only catches a negation spelled
/// out as its own word.
fn has_negation(content: &serde_json::Value) -> bool {
    const NEGATION_WORDS: &[&str] = &[
        "not", "never", "no", "none", "nothing", "nobody", "nowhere", "cannot",
    ];
    let text = match content {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    text.split(|c: char| !c.is_alphanumeric())
        .any(|w| NEGATION_WORDS.contains(&w.to_lowercase().as_str()))
}

/// The Jaccard similarity of two content values' significant words:
/// lowercased, split on non-alphanumeric characters, a short stopword list
/// dropped. Crude on purpose - it never looks at meaning, only shared
/// surface words.
fn word_overlap(a: &serde_json::Value, b: &serde_json::Value) -> f64 {
    let words_a = significant_words(a);
    let words_b = significant_words(b);
    if words_a.is_empty() || words_b.is_empty() {
        return 0.0;
    }
    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();
    intersection as f64 / union as f64
}

fn significant_words(content: &serde_json::Value) -> BTreeSet<String> {
    const STOPWORDS: &[&str] = &[
        "a", "an", "the", "is", "are", "was", "were", "be", "been", "has", "have", "had", "on",
        "at", "to", "of", "and", "or", "in", "it", "this", "that", "as", "by", "for", "with",
    ];
    let text = match content {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .filter(|w| !STOPWORDS.contains(&w.as_str()))
        .collect()
}

/// Print a completion script for `shell` to stdout. Does not need a store.
fn cmd_completions(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}

fn cmd_export(output: Option<PathBuf>) -> Result<()> {
    let store = open_store()?;
    let export = store.export()?;
    let json = serde_json::to_string_pretty(&export).context("encoding the export as JSON")?;
    match output {
        Some(path) => {
            fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
            eprintln!(
                "Exported {} object(s) to {}",
                export.objects.len(),
                path.display()
            );
        }
        None => println!("{json}"),
    }
    Ok(())
}

fn cmd_import(file: PathBuf, path: Option<PathBuf>) -> Result<()> {
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
    let export: Export = serde_json::from_str(&text)
        .with_context(|| format!("{} is not a valid mnem export", file.display()))?;
    let path = path.unwrap_or_else(|| PathBuf::from("."));
    let object_count = export.objects.len();
    let store = Store::import(&path, &export)?;
    println!(
        "Imported {object_count} object(s) into {}",
        store.mnem_dir().display()
    );
    Ok(())
}

/// The registry folder: the explicit `--registry`, falling back to
/// `$MNEM_REGISTRY`, same fallback shape `--author` already uses for
/// `$MNEM_AUTHOR`. No invented default directory - a spike this thin
/// should never write somewhere the caller didn't ask for.
fn registry_dir(explicit: Option<PathBuf>) -> Result<PathBuf> {
    explicit
        .or_else(|| std::env::var_os("MNEM_REGISTRY").map(PathBuf::from))
        .context("no registry: pass --registry <DIR> or set $MNEM_REGISTRY")
}

/// Every bundle name (the file stem of each `*.json`) in `dir`, sorted.
/// Shared by `list` and `render` - both just read the folder.
fn registry_bundle_names(dir: &PathBuf) -> Result<Vec<String>> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            (path.extension().and_then(|e| e.to_str()) == Some("json"))
                .then(|| path.file_stem()?.to_str().map(str::to_string))
                .flatten()
        })
        .collect();
    names.sort();
    Ok(names)
}

/// Export the current store and write it to `<registry>/<name>.json`.
/// `list` and `fork` below are the exact reverse of this and of each
/// other - the registry is just a folder, `publish`/`fork` are
/// `export`/`import` under a name instead of an arbitrary path.
fn cmd_registry_publish(name: String, registry: Option<PathBuf>) -> Result<()> {
    let dir = registry_dir(registry)?;
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let store = open_store()?;
    let export = store.export()?;
    let json = serde_json::to_string_pretty(&export).context("encoding the export as JSON")?;
    let path = dir.join(format!("{name}.json"));
    fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
    println!(
        "Published {} ({} object(s)) to {}",
        style::good(&name),
        export.objects.len(),
        path.display()
    );
    Ok(())
}

fn cmd_registry_list(registry: Option<PathBuf>) -> Result<()> {
    let dir = registry_dir(registry)?;
    if !dir.exists() {
        println!("no registry at {}", dir.display());
        return Ok(());
    }
    let names = registry_bundle_names(&dir)?;
    if names.is_empty() {
        println!("registry is empty ({})", dir.display());
        return Ok(());
    }
    for name in names {
        println!("{}", style::branch(&name));
    }
    Ok(())
}

fn cmd_registry_fork(name: String, dest: PathBuf, registry: Option<PathBuf>) -> Result<()> {
    let dir = registry_dir(registry)?;
    let path = dir.join(format!("{name}.json"));
    let text = fs::read_to_string(&path)
        .with_context(|| format!("no bundle {name:?} in the registry ({})", path.display()))?;
    let export: Export = serde_json::from_str(&text)
        .with_context(|| format!("{} is not a valid mnem export", path.display()))?;
    let object_count = export.objects.len();
    let store = Store::import(&dest, &export)?;
    println!(
        "Forked {} into {} ({object_count} object(s))",
        style::good(&name),
        store.mnem_dir().display()
    );
    Ok(())
}

/// Render every bundle in the registry as one static HTML page: name,
/// prompt summary, tool count. No server, build-time only (#453). Reads
/// each bundle the same way `fork` does - `Store::import` into a scratch
/// directory under the OS temp dir, read, then thrown away - rather than
/// hand-parsing the export's own object graph or adding a dependency to
/// do it. `agent:prompt`/`agent:tools` are #450's own demo node ids, not
/// a real convention, so a bundle missing either just reads "(none)".
fn cmd_registry_render(registry: Option<PathBuf>, output: PathBuf) -> Result<()> {
    let dir = registry_dir(registry)?;
    if !dir.exists() {
        anyhow::bail!("no registry at {}", dir.display());
    }
    let names = registry_bundle_names(&dir)?;

    let mut rows = String::new();
    for name in &names {
        let path = dir.join(format!("{name}.json"));
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let export: Export = serde_json::from_str(&text)
            .with_context(|| format!("{} is not a valid mnem export", path.display()))?;

        let scratch = std::env::temp_dir().join(format!("mnem-registry-render-{name}"));
        let _ = fs::remove_dir_all(&scratch);
        let store =
            Store::import(&scratch, &export).with_context(|| format!("reading bundle {name:?}"))?;
        let prompt = store.working_node("agent:prompt")?;
        let tools = store.working_node("agent:tools")?;
        let _ = fs::remove_dir_all(&scratch);

        let prompt_summary = match prompt.map(|n| n.content) {
            Some(serde_json::Value::String(s)) => truncate(&s, 80),
            Some(_) => "(agent:prompt is not a string)".to_string(),
            None => "(none)".to_string(),
        };
        let tool_count = match tools.map(|n| n.content) {
            Some(serde_json::Value::Array(a)) => a.len().to_string(),
            Some(_) => "(agent:tools is not a list)".to_string(),
            None => "(none)".to_string(),
        };

        rows.push_str(&format!(
            "<tr>\n  <td>{}</td>\n  <td>{}</td>\n  <td>{}</td>\n</tr>\n",
            escape_html(name),
            escape_html(&prompt_summary),
            escape_html(&tool_count),
        ));
    }

    let page = format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>mnem registry: {count} bundle(s)</title>
<style>
body {{ font-family: -apple-system, sans-serif; background: #1e1e1e; color: #ddd; margin: 2rem; }}
h1 {{ font-size: 1.1rem; font-weight: 600; }}
table {{ border-collapse: collapse; width: 100%; margin-top: 1rem; }}
th, td {{ border: 1px solid #444; padding: 0.5rem; vertical-align: top; text-align: left; }}
th {{ background: #272727; font-family: monospace; font-size: 0.85rem; }}
td:first-child {{ font-family: monospace; }}
</style>
</head>
<body>
<h1>Registry: <code>{dir}</code>, {count} bundle(s)</h1>
<table>
<tr><th>name</th><th>prompt summary</th><th>tool count</th></tr>
{rows}</table>
</body>
</html>
"#,
        count = names.len(),
        dir = escape_html(&dir.display().to_string()),
    );
    fs::write(&output, page).with_context(|| format!("writing {}", output.display()))?;
    println!(
        "Wrote {} ({} bundle(s)) to {}",
        style::good("the registry page"),
        names.len(),
        output.display()
    );
    Ok(())
}

/// Truncate `s` to at most `max` characters, appending `...` if cut.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let short: String = s.chars().take(max).collect();
        format!("{short}...")
    }
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
