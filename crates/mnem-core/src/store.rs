//! [`Store`], the on-disk `.mnem/` directory (ADR-0002).
//!
//! ```text
//! .mnem/
//!   store.redb   the object and ref tables
//!   HEAD         the current pointer, one line of plain text
//!   config       key = value lines
//! ```
//!
//! [`Store::init`] creates one; [`Store::open`] finds the nearest one at or
//! above a path. This module owns the store's lifecycle, its `config`, and
//! `HEAD` (a text file). The `refs` table is in [`refs`](crate::refs), the
//! object table in [`objects`](crate::objects).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::{MnemError, Result};
use crate::head::Head;
use crate::id::ObjectId;

/// The directory name inside the store root.
pub const STORE_DIR: &str = ".mnem";
/// The `redb` file inside [`STORE_DIR`].
pub const DB_FILE: &str = "store.redb";
/// The `HEAD` file inside [`STORE_DIR`].
pub const HEAD_FILE: &str = "HEAD";
/// The `config` file inside [`STORE_DIR`].
pub const CONFIG_FILE: &str = "config";

fn io<E: std::fmt::Display>(e: E) -> MnemError {
    MnemError::Io(e.to_string())
}

/// An open store.
pub struct Store {
    mnem_dir: PathBuf,
    config: Config,
    db: redb::Database,
}

impl Store {
    /// Create a store in `root`, at `root/.mnem/`.
    ///
    /// Fails if `root` already holds a store, or if any ancestor of `root`
    /// does: a store inside a store is refused.
    pub fn init(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let mnem_dir = root.join(STORE_DIR);

        if mnem_dir.exists() {
            return Err(MnemError::StoreExists(mnem_dir));
        }
        if let Some(ancestor) = find_store_dir(root.parent()) {
            return Err(MnemError::StoreExists(ancestor));
        }

        fs::create_dir_all(&mnem_dir).map_err(io)?;

        let config = Config::default();
        write_atomic(
            &mnem_dir.join(CONFIG_FILE),
            config.to_file_string().as_bytes(),
        )?;
        write_atomic(
            &mnem_dir.join(HEAD_FILE),
            format!("ref: {}\n", config.default_branch).as_bytes(),
        )?;

        let db = redb::Database::create(mnem_dir.join(DB_FILE)).map_err(io)?;

        Ok(Self {
            mnem_dir,
            config,
            db,
        })
    }

    /// Open the nearest store at or above `start`.
    pub fn open(start: impl AsRef<Path>) -> Result<Self> {
        let start = start.as_ref();
        let mnem_dir =
            find_store_dir(Some(start)).ok_or_else(|| MnemError::NoStore(start.to_path_buf()))?;

        let config_text = fs::read_to_string(mnem_dir.join(CONFIG_FILE)).map_err(io)?;
        let config = Config::parse(&config_text)?;

        let db = redb::Database::open(mnem_dir.join(DB_FILE)).map_err(io)?;

        Ok(Self {
            mnem_dir,
            config,
            db,
        })
    }

    /// The `.mnem/` directory.
    pub fn mnem_dir(&self) -> &Path {
        &self.mnem_dir
    }

    /// The store root, the parent of `.mnem/`.
    pub fn root(&self) -> &Path {
        self.mnem_dir.parent().unwrap_or(&self.mnem_dir)
    }

    /// The parsed `config`.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Begin a read transaction over the object and ref tables.
    pub fn begin_read(&self) -> Result<redb::ReadTransaction> {
        self.db.begin_read().map_err(io)
    }

    /// Begin a write transaction. A commit is one such transaction (ADR-0008).
    pub fn begin_write(&self) -> Result<redb::WriteTransaction> {
        self.db.begin_write().map_err(io)
    }

    /// Read and parse `HEAD`.
    pub fn head(&self) -> Result<Head> {
        let text = fs::read_to_string(self.mnem_dir.join(HEAD_FILE)).map_err(io)?;
        Head::parse(&text)
    }

    /// Write `HEAD` atomically. An attached `HEAD`'s branch name is validated.
    pub fn set_head(&self, head: &Head) -> Result<()> {
        if let Head::Attached(name) = head {
            crate::refs::validate_name(name)?;
        }
        write_atomic(
            &self.mnem_dir.join(HEAD_FILE),
            head.to_file_string().as_bytes(),
        )
    }

    /// The commit `HEAD` currently points at, resolving an attached branch
    /// through `txn`. `None` when attached to a branch that does not exist yet.
    pub fn resolve_head(&self, txn: &redb::ReadTransaction) -> Result<Option<ObjectId>> {
        match self.head()? {
            Head::Detached(id) => Ok(Some(id)),
            Head::Attached(name) => crate::refs::get(txn, &name),
        }
    }

    /// Load a memory node by its object id.
    pub fn node(&self, object_id: ObjectId) -> Result<crate::object::MemoryNode> {
        let txn = self.begin_read()?;
        crate::objects::require_node(&txn, object_id)
    }

    /// Resolve a commit-ish (ADR-0012): an exact branch name first, then an
    /// unambiguous lowercase-hex commit id prefix of 4 to 64 characters.
    pub fn resolve_commitish(&self, spec: &str) -> Result<ObjectId> {
        let txn = self.begin_read()?;
        self.resolve_commitish_in(&txn, spec)
    }

    /// [`resolve_commitish`](Self::resolve_commitish) inside a transaction the
    /// caller owns.
    pub fn resolve_commitish_in(
        &self,
        txn: &redb::ReadTransaction,
        spec: &str,
    ) -> Result<ObjectId> {
        if let Some(id) = crate::refs::get(txn, spec)? {
            return Ok(id);
        }

        let is_hex_prefix = (4..=ObjectId::LEN * 2).contains(&spec.len())
            && spec.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        if !is_hex_prefix {
            return Err(MnemError::InvalidRef(format!(
                "no branch named {spec:?}, and it is not a commit id prefix (4 to 64 lowercase hex)"
            )));
        }

        let mut commits: Vec<ObjectId> = Vec::new();
        for id in crate::objects::ids_with_prefix(txn, spec)? {
            if let Some(crate::object::Object::Commit(_)) = crate::objects::get(txn, id)? {
                commits.push(id);
            }
        }
        match commits.as_slice() {
            [] => Err(MnemError::InvalidRef(format!(
                "no commit matching {spec:?}"
            ))),
            [only] => Ok(*only),
            many => Err(MnemError::InvalidRef(format!(
                "ambiguous commit prefix {spec:?}: {} commits match",
                many.len()
            ))),
        }
    }
}

/// Walk up from `start` (inclusive) looking for a directory that contains a
/// `.mnem/` subdirectory. Returns the path to that `.mnem/`.
fn find_store_dir(start: Option<&Path>) -> Option<PathBuf> {
    let mut dir = start;
    while let Some(d) = dir {
        let candidate = d.join(STORE_DIR);
        if candidate.is_dir() {
            return Some(candidate);
        }
        dir = d.parent();
    }
    None
}

/// Write `contents` to `path` by writing a sibling temp file and renaming it,
/// so a reader never sees a half-written file.
fn write_atomic(path: &Path, contents: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut file = fs::File::create(&tmp).map_err(io)?;
        file.write_all(contents).map_err(io)?;
        file.sync_all().map_err(io)?;
    }
    fs::rename(&tmp, path).map_err(io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "mnem-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn init_creates_the_layout() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();

        assert!(root.join(".mnem/store.redb").is_file());
        assert_eq!(
            fs::read_to_string(root.join(".mnem/HEAD")).unwrap(),
            "ref: main\n"
        );
        assert_eq!(store.config().format_version, 1);
        assert_eq!(store.root(), root);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn init_refuses_a_store_inside_a_store() {
        let root = tempdir();
        Store::init(&root).unwrap();

        let nested = root.join("sub");
        fs::create_dir_all(&nested).unwrap();
        assert!(matches!(
            Store::init(&nested),
            Err(MnemError::StoreExists(_))
        ));
        assert!(matches!(Store::init(&root), Err(MnemError::StoreExists(_))));

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn open_discovers_from_a_subdirectory() {
        let root = tempdir();
        Store::init(&root).unwrap();

        let deep = root.join("a/b/c");
        fs::create_dir_all(&deep).unwrap();
        let store = Store::open(&deep).unwrap();
        assert_eq!(store.root(), root);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn open_fails_where_there_is_no_store() {
        let root = tempdir();
        assert!(matches!(Store::open(&root), Err(MnemError::NoStore(_))));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn transactions_open() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();
        store.begin_read().unwrap();
        let w = store.begin_write().unwrap();
        w.commit().unwrap();
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn head_starts_attached_to_a_missing_main_then_resolves() {
        let root = tempdir();
        let store = Store::init(&root).unwrap();

        assert_eq!(store.head().unwrap(), Head::Attached("main".to_string()));

        // A fresh store's HEAD resolves to nothing: main does not exist yet.
        {
            let txn = store.begin_read().unwrap();
            assert_eq!(store.resolve_head(&txn).unwrap(), None);
        }

        // Point main at a commit, and HEAD now resolves to it.
        let target = ObjectId::hash_canonical(b"a commit");
        {
            let txn = store.begin_write().unwrap();
            crate::refs::set(&txn, "main", target).unwrap();
            txn.commit().unwrap();
        }
        {
            let txn = store.begin_read().unwrap();
            assert_eq!(store.resolve_head(&txn).unwrap(), Some(target));
        }

        // Detach HEAD onto a specific commit.
        store.set_head(&Head::Detached(target)).unwrap();
        assert_eq!(store.head().unwrap(), Head::Detached(target));
        {
            let txn = store.begin_read().unwrap();
            assert_eq!(store.resolve_head(&txn).unwrap(), Some(target));
        }

        fs::remove_dir_all(&root).ok();
    }
}
