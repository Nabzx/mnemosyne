//! `_mnem`, the Python extension module for Mnemosyne.
//!
//! This is a thin binding over `mnem-core` (ADR-0004): it exposes the store and
//! the commit graph to Python and maps [`mnem_core::MnemError`] onto a Python
//! exception hierarchy. It carries no operation semantics of its own. The
//! ergonomic layer (context managers, dataclasses) is the `mnem` Python package
//! built on top of this, see #30.

use std::cell::RefCell;
use std::collections::BTreeMap;

use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};

use mnem_core::{
    ChangeKind, Checkout, ConflictKind, ContentKind, DiffTarget, MemoryNode, MergeOutcome,
    MergeStrategy, MnemError, NodeChange, ObjectId, Provenance, Resolution, Store,
};

create_exception!(
    _mnem,
    MnemErrorException,
    PyException,
    "Base class for every error raised by the Mnemosyne core."
);
create_exception!(
    _mnem,
    NotFoundError,
    MnemErrorException,
    "An object was asked for by id and is not in the store."
);
create_exception!(
    _mnem,
    InvalidRefError,
    MnemErrorException,
    "A ref name does not resolve, or is not a legal name."
);
create_exception!(
    _mnem,
    ConflictError,
    MnemErrorException,
    "A ref moved under a compare-and-swap update."
);
create_exception!(
    _mnem,
    CorruptStoreError,
    MnemErrorException,
    "Stored bytes could not be decoded, or an id is dangling."
);
create_exception!(
    _mnem,
    FormatVersionError,
    MnemErrorException,
    "The store's format_version is newer than this release supports."
);
create_exception!(
    _mnem,
    StoreExistsError,
    MnemErrorException,
    "init was asked to create a store where one already exists."
);
create_exception!(
    _mnem,
    NoStoreError,
    MnemErrorException,
    "open found no store at or above the given path."
);
create_exception!(
    _mnem,
    StoreIoError,
    MnemErrorException,
    "An error from the underlying storage."
);

/// Map a core error onto the matching Python exception (ADR-0004). Every
/// `MnemError` variant has a row here; the enum is `#[non_exhaustive]`, so the
/// wildcard falls back to the base class.
fn to_py_err(error: MnemError) -> PyErr {
    let message = error.to_string();
    match error {
        MnemError::NotFound(_) => NotFoundError::new_err(message),
        MnemError::InvalidRef(_) => InvalidRefError::new_err(message),
        MnemError::Conflict(_) => ConflictError::new_err(message),
        MnemError::CorruptStore(_) => CorruptStoreError::new_err(message),
        MnemError::FormatVersion { .. } => FormatVersionError::new_err(message),
        MnemError::StoreExists(_) => StoreExistsError::new_err(message),
        MnemError::NoStore(_) => NoStoreError::new_err(message),
        MnemError::Io(_) => StoreIoError::new_err(message),
        _ => MnemErrorException::new_err(message),
    }
}

fn parse_content_kind(name: &str) -> PyResult<ContentKind> {
    match name {
        "note" => Ok(ContentKind::Note),
        "claim" => Ok(ContentKind::Claim),
        other => Err(PyValueError::new_err(format!(
            "content_kind must be \"note\" or \"claim\", got {other:?}"
        ))),
    }
}

fn content_kind_name(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Note => "note",
        ContentKind::Claim => "claim",
    }
}

/// A memory node as a dict: `content` is a JSON string, `provenance` its own
/// dict. The `mnem` package turns this back into a `MemoryNode` dataclass.
fn node_to_dict<'py>(py: Python<'py>, node: &MemoryNode) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    dict.set_item("id", &node.id)?;
    dict.set_item(
        "content",
        serde_json::to_string(&node.content).unwrap_or_default(),
    )?;
    dict.set_item("content_kind", content_kind_name(node.content_kind))?;
    dict.set_item("event_time", node.event_time)?;

    let prov = PyDict::new(py);
    prov.set_item("agent_step", &node.provenance.agent_step)?;
    prov.set_item("observation", &node.provenance.observation)?;
    prov.set_item("tool_call", &node.provenance.tool_call)?;
    prov.set_item("source", &node.provenance.source)?;
    prov.set_item("note", &node.provenance.note)?;
    dict.set_item("provenance", prov)?;

    Ok(dict)
}

/// A Mnemosyne store. Wraps [`mnem_core::Store`].
#[pyclass(name = "Store", frozen)]
struct PyStore {
    inner: Store,
}

#[pymethods]
impl PyStore {
    /// Create a store at `path/.mnem/` and open it.
    #[staticmethod]
    fn init(path: &str) -> PyResult<Self> {
        let inner = Store::init(path).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// Open the nearest store at or above `path`.
    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> {
        let inner = Store::open(path).map_err(to_py_err)?;
        Ok(Self { inner })
    }

    /// The store root, the directory that holds `.mnem/`.
    fn root(&self) -> String {
        self.inner.root().display().to_string()
    }

    /// The store's on-disk `format_version`.
    fn format_version(&self) -> u32 {
        self.inner.config().format_version
    }

    /// The raw contents of `HEAD`, for example `"ref: main"`.
    fn head(&self) -> PyResult<String> {
        Ok(self
            .inner
            .head()
            .map_err(to_py_err)?
            .to_file_string()
            .trim_end()
            .to_string())
    }

    /// Stage a memory node for the next commit. `content` is a JSON string; a
    /// plain note is a JSON string literal. The provenance fields match
    /// `mnem_core::Provenance` (ADR-0003). Returns the node object's id as hex.
    #[pyo3(signature = (
        id,
        content,
        *,
        content_kind = "note",
        agent_step = None,
        observation = None,
        tool_call = None,
        source = None,
        note = None,
        event_time = None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn add(
        &self,
        id: &str,
        content: &str,
        content_kind: &str,
        agent_step: Option<String>,
        observation: Option<String>,
        tool_call: Option<String>,
        source: Option<String>,
        note: Option<String>,
        event_time: Option<i64>,
    ) -> PyResult<String> {
        let value = serde_json::from_str(content)
            .map_err(|e| PyValueError::new_err(format!("content is not valid JSON: {e}")))?;
        let node = MemoryNode {
            id: id.to_string(),
            content: value,
            content_kind: parse_content_kind(content_kind)?,
            provenance: Provenance {
                agent_step,
                observation,
                tool_call,
                source,
                note,
            },
            event_time,
        };
        let object_id = self.inner.stage(&node).map_err(to_py_err)?;
        Ok(object_id.to_hex())
    }

    /// The staged nodes as `(node_id, object_id_hex)` pairs, sorted by node id.
    fn staged(&self) -> PyResult<Vec<(String, String)>> {
        Ok(self
            .inner
            .staged()
            .map_err(to_py_err)?
            .into_iter()
            .map(|(node_id, object_id)| (node_id, object_id.to_hex()))
            .collect())
    }

    /// Drop a node from staging. Returns whether it was staged.
    fn unstage(&self, node_id: &str) -> PyResult<bool> {
        self.inner.unstage(node_id).map_err(to_py_err)
    }

    /// Record everything staged as a commit on the current branch. `time_ms` is
    /// the record time in Unix milliseconds. Returns the commit id as hex.
    fn commit(&self, message: &str, author: &str, time_ms: i64) -> PyResult<String> {
        let id = self
            .inner
            .commit(message, author, time_ms)
            .map_err(to_py_err)?;
        Ok(id.to_hex())
    }

    /// The commit history from `HEAD`, newest first. Each entry is a dict with
    /// `id`, `parents`, `state`, `message`, `author` and `time`.
    #[pyo3(signature = (*, first_parent = false, limit = None))]
    fn log<'py>(
        &self,
        py: Python<'py>,
        first_parent: bool,
        limit: Option<usize>,
    ) -> PyResult<Bound<'py, PyList>> {
        let entries = self
            .inner
            .log_head(first_parent, limit)
            .map_err(to_py_err)?;
        let out = PyList::empty(py);
        for (id, commit) in entries {
            let row = PyDict::new(py);
            row.set_item("id", id.to_hex())?;
            let parents: Vec<String> = commit.parents.iter().map(|p| p.to_hex()).collect();
            row.set_item("parents", parents)?;
            row.set_item("state", commit.state.to_hex())?;
            row.set_item("message", commit.message)?;
            row.set_item("author", commit.author)?;
            row.set_item("time", commit.time)?;
            out.append(row)?;
        }
        Ok(out)
    }

    // --- Phase 2 (ADR-0012) ---

    /// The staged deletions (`mnem rm`), sorted by node id.
    fn staged_deletions(&self) -> PyResult<Vec<String>> {
        self.inner.staged_deletions().map_err(to_py_err)
    }

    /// Stage the deletion of a node from the next commit.
    fn rm(&self, id: &str) -> PyResult<()> {
        self.inner.rm(id).map_err(to_py_err)
    }

    /// The commit `HEAD` resolves to, as hex, or `None` on an unborn branch.
    fn head_commit(&self) -> PyResult<Option<String>> {
        Ok(self
            .inner
            .head_commit()
            .map_err(to_py_err)?
            .map(|id| id.to_hex()))
    }

    /// Resolve a commit-ish (branch name or commit id prefix) to a commit hex.
    fn resolve_commitish(&self, spec: &str) -> PyResult<String> {
        Ok(self
            .inner
            .resolve_commitish(spec)
            .map_err(to_py_err)?
            .to_hex())
    }

    /// Create a branch at `start` (a commit-ish) or at `HEAD`.
    #[pyo3(signature = (name, start = None))]
    fn branch(&self, name: &str, start: Option<&str>) -> PyResult<()> {
        self.inner.branch(name, start).map_err(to_py_err)
    }

    /// Every branch as `(name, tip_hex)`, sorted by name.
    fn branches(&self) -> PyResult<Vec<(String, String)>> {
        Ok(self
            .inner
            .branches()
            .map_err(to_py_err)?
            .into_iter()
            .map(|(name, tip)| (name, tip.to_hex()))
            .collect())
    }

    /// Delete a branch. Returns whether it existed. Refuses the current branch.
    fn delete_branch(&self, name: &str) -> PyResult<bool> {
        self.inner.delete_branch(name).map_err(to_py_err)
    }

    /// Move `HEAD` to `target`. Returns `(kind, ref)` where `kind` is
    /// `"branch"`, `"detached"` or `"already"`.
    #[pyo3(signature = (target, *, discard = false))]
    fn checkout(&self, target: &str, discard: bool) -> PyResult<(String, String)> {
        Ok(
            match self.inner.checkout(target, discard).map_err(to_py_err)? {
                Checkout::SwitchedToBranch(name) => ("branch".to_string(), name),
                Checkout::DetachedAt(id) => ("detached".to_string(), id.to_hex()),
                Checkout::AlreadyThere => ("already".to_string(), String::new()),
            },
        )
    }

    /// The current working memory as `{node_id: node_dict}`.
    fn working_memory<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let out = PyDict::new(py);
        for (id, node) in self.inner.working_memory().map_err(to_py_err)? {
            out.set_item(id, node_to_dict(py, &node)?)?;
        }
        Ok(out)
    }

    /// One node from working memory, or `None` if absent or tombstoned.
    fn working_node<'py>(&self, py: Python<'py>, id: &str) -> PyResult<Option<Bound<'py, PyDict>>> {
        Ok(match self.inner.working_node(id).map_err(to_py_err)? {
            Some(node) => Some(node_to_dict(py, &node)?),
            None => None,
        })
    }

    /// The full memory at a commit (a commit-ish), as `{node_id: node_dict}`.
    fn state_at<'py>(&self, py: Python<'py>, commit: &str) -> PyResult<Bound<'py, PyDict>> {
        let id = self.inner.resolve_commitish(commit).map_err(to_py_err)?;
        let out = PyDict::new(py);
        for (node_id, node) in self.inner.state_at(id).map_err(to_py_err)? {
            out.set_item(node_id, node_to_dict(py, &node)?)?;
        }
        Ok(out)
    }

    /// The changes between two states. `from_ref` defaults to the `HEAD`
    /// commit, `to_ref` to working memory. Each change is a dict with `id`,
    /// `kind` (`"added"` / `"removed"` / `"modified"`) and `old` / `new` node
    /// dicts (or `None`).
    #[pyo3(signature = (from_ref = None, to_ref = None))]
    fn diff<'py>(
        &self,
        py: Python<'py>,
        from_ref: Option<&str>,
        to_ref: Option<&str>,
    ) -> PyResult<Bound<'py, PyList>> {
        let from = self.diff_target(from_ref, true)?;
        let to = self.diff_target(to_ref, false)?;

        let out = PyList::empty(py);
        for change in self.inner.diff(from, to).map_err(to_py_err)? {
            let row = PyDict::new(py);
            row.set_item("id", change.id())?;
            match change {
                NodeChange::Added { new, .. } => {
                    row.set_item("kind", "added")?;
                    row.set_item("old", py.None())?;
                    row.set_item("new", self.node_dict(py, new)?)?;
                }
                NodeChange::Removed { old, .. } => {
                    row.set_item("kind", "removed")?;
                    row.set_item("old", self.node_dict(py, old)?)?;
                    row.set_item("new", py.None())?;
                }
                NodeChange::Modified { old, new, .. } => {
                    row.set_item("kind", "modified")?;
                    row.set_item("old", self.node_dict(py, old)?)?;
                    row.set_item("new", self.node_dict(py, new)?)?;
                }
            }
            out.append(row)?;
        }
        Ok(out)
    }

    /// Merge `theirs` (a commit-ish) into the current branch. `resolutions` maps
    /// a conflicting node id to `"ours"` / `"theirs"` / `"base"` / `"delete"`,
    /// or to a node dict for a `set` resolution. `strategy` is `"ours"` or
    /// `"theirs"`. Returns a dict with `status`
    /// (`"up-to-date"` / `"fast-forwarded"` / `"merged"` / `"conflicts"`),
    /// `commit`, and `conflicts`.
    #[pyo3(signature = (theirs, *, resolutions = None, strategy = None, message = None, author = "unknown", time_ms = 0))]
    #[allow(clippy::too_many_arguments)]
    fn merge<'py>(
        &self,
        py: Python<'py>,
        theirs: &str,
        resolutions: Option<&Bound<'py, PyDict>>,
        strategy: Option<&str>,
        message: Option<&str>,
        author: &str,
        time_ms: i64,
    ) -> PyResult<Bound<'py, PyDict>> {
        let resolutions = parse_resolutions(resolutions)?;
        let strategy = match strategy {
            None => None,
            Some("ours") => Some(MergeStrategy::Ours),
            Some("theirs") => Some(MergeStrategy::Theirs),
            Some(other) => {
                return Err(PyValueError::new_err(format!(
                    "strategy must be \"ours\" or \"theirs\", got {other:?}"
                )));
            }
        };

        let outcome = self
            .inner
            .merge(theirs, &resolutions, strategy, message, author, time_ms)
            .map_err(to_py_err)?;

        let out = PyDict::new(py);
        out.set_item("conflicts", PyList::empty(py))?;
        match outcome {
            MergeOutcome::AlreadyUpToDate => {
                out.set_item("status", "up-to-date")?;
                out.set_item("commit", py.None())?;
            }
            MergeOutcome::FastForwarded(id) => {
                out.set_item("status", "fast-forwarded")?;
                out.set_item("commit", id.to_hex())?;
            }
            MergeOutcome::Merged(id) => {
                out.set_item("status", "merged")?;
                out.set_item("commit", id.to_hex())?;
            }
            MergeOutcome::Conflicts(conflicts) => {
                out.set_item("status", "conflicts")?;
                out.set_item("commit", py.None())?;
                let list = PyList::empty(py);
                for conflict in conflicts {
                    let row = PyDict::new(py);
                    row.set_item("id", &conflict.id)?;
                    row.set_item("kind", conflict_kind_name(conflict.kind))?;
                    row.set_item("base", self.opt_node_dict(py, conflict.base)?)?;
                    row.set_item("ours", self.opt_node_dict(py, conflict.ours)?)?;
                    row.set_item("theirs", self.opt_node_dict(py, conflict.theirs)?)?;
                    list.append(row)?;
                }
                out.set_item("conflicts", list)?;
            }
        }
        Ok(out)
    }

    /// Blame `node_id` to the commit that set its current value as of `at` (a
    /// commit-ish; `None` is `HEAD`). Returns `{commit, node, time}`; raises
    /// `InvalidRefError` if the node is absent at `at`.
    #[pyo3(signature = (node_id, at = None))]
    fn blame<'py>(
        &self,
        py: Python<'py>,
        node_id: &str,
        at: Option<&str>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let blame = self.inner.blame(node_id, at).map_err(to_py_err)?;
        let out = PyDict::new(py);
        out.set_item("commit", blame.commit.to_hex())?;
        out.set_item("node", node_to_dict(py, &blame.node)?)?;
        out.set_item("time", blame.time)?;
        Ok(out)
    }

    /// Bisect for the first commit where `predicate` holds. `predicate` is
    /// called with `{node_id: node_dict}` for the memory at a commit. `bad`
    /// defaults to `HEAD`, `good` to the first-parent chain's root. Returns the
    /// commit hex.
    #[pyo3(signature = (predicate, *, bad = None, good = None))]
    fn bisect(
        &self,
        py: Python<'_>,
        predicate: &Bound<'_, PyAny>,
        bad: Option<&str>,
        good: Option<&str>,
    ) -> PyResult<String> {
        let captured: RefCell<Option<PyErr>> = RefCell::new(None);
        let call = |state: &BTreeMap<String, MemoryNode>| -> bool {
            if captured.borrow().is_some() {
                return false;
            }
            let dict = PyDict::new(py);
            for (id, node) in state {
                match node_to_dict(py, node).and_then(|d| dict.set_item(id, d)) {
                    Ok(()) => {}
                    Err(e) => {
                        *captured.borrow_mut() = Some(e);
                        return false;
                    }
                }
            }
            match predicate.call1((dict,)).and_then(|r| r.is_truthy()) {
                Ok(truthy) => truthy,
                Err(e) => {
                    *captured.borrow_mut() = Some(e);
                    false
                }
            }
        };

        let result = self.inner.bisect(bad, good, call);
        if let Some(e) = captured.into_inner() {
            return Err(e);
        }
        result.map(|id| id.to_hex()).map_err(to_py_err)
    }

    /// The node ids a commit changed against its first parent, as
    /// `{node_id: "added" | "modified" | "removed"}`.
    fn changed_by<'py>(&self, py: Python<'py>, commit: &str) -> PyResult<Bound<'py, PyDict>> {
        let id = self.inner.resolve_commitish(commit).map_err(to_py_err)?;
        let out = PyDict::new(py);
        for (node_id, kind) in self.inner.changed_by(id).map_err(to_py_err)? {
            out.set_item(node_id, change_kind_name(kind))?;
        }
        Ok(out)
    }
}

fn change_kind_name(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "added",
        ChangeKind::Modified => "modified",
        ChangeKind::Removed => "removed",
    }
}

fn conflict_kind_name(kind: ConflictKind) -> &'static str {
    match kind {
        ConflictKind::EditEdit => "edit/edit",
        ConflictKind::DeleteEdit => "delete/edit",
        ConflictKind::EditDelete => "edit/delete",
        ConflictKind::AddAdd => "add/add",
    }
}

fn parse_resolutions(
    resolutions: Option<&Bound<'_, PyDict>>,
) -> PyResult<std::collections::BTreeMap<String, Resolution>> {
    let mut out = std::collections::BTreeMap::new();
    let Some(dict) = resolutions else {
        return Ok(out);
    };
    for (key, value) in dict.iter() {
        let id: String = key.extract()?;
        let resolution = if let Ok(side) = value.extract::<String>() {
            match side.as_str() {
                "ours" => Resolution::Ours,
                "theirs" => Resolution::Theirs,
                "base" => Resolution::Base,
                "delete" => Resolution::Delete,
                other => {
                    return Err(PyValueError::new_err(format!(
                        "resolution for {id:?} must be \"ours\", \"theirs\", \"base\", \"delete\", or a node dict, got {other:?}"
                    )));
                }
            }
        } else {
            let node_dict: Bound<'_, PyDict> = value.extract()?;
            Resolution::Set(node_from_dict(id.as_str(), &node_dict)?)
        };
        out.insert(id, resolution);
    }
    Ok(out)
}

fn node_from_dict(id: &str, dict: &Bound<'_, PyDict>) -> PyResult<MemoryNode> {
    let content_str: String = dict
        .get_item("content")?
        .ok_or_else(|| PyValueError::new_err("a set resolution needs a \"content\" key"))?
        .extract()?;
    let content = serde_json::from_str(&content_str)
        .map_err(|e| PyValueError::new_err(format!("content is not valid JSON: {e}")))?;
    let content_kind = match dict.get_item("content_kind")? {
        Some(v) => parse_content_kind(&v.extract::<String>()?)?,
        None => ContentKind::Note,
    };
    Ok(MemoryNode {
        id: id.to_string(),
        content,
        content_kind,
        provenance: Provenance::default(),
        event_time: None,
    })
}

impl PyStore {
    fn opt_node_dict<'py>(
        &self,
        py: Python<'py>,
        object_id: Option<ObjectId>,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        match object_id {
            Some(id) => Ok(Some(self.node_dict(py, id)?)),
            None => Ok(None),
        }
    }

    fn diff_target(&self, spec: Option<&str>, is_from: bool) -> PyResult<DiffTarget> {
        match spec {
            Some(s) => Ok(DiffTarget::Commit(
                self.inner.resolve_commitish(s).map_err(to_py_err)?,
            )),
            None if is_from => Ok(match self.inner.head_commit().map_err(to_py_err)? {
                Some(id) => DiffTarget::Commit(id),
                None => DiffTarget::Empty,
            }),
            None => Ok(DiffTarget::Working),
        }
    }

    fn node_dict<'py>(&self, py: Python<'py>, object_id: ObjectId) -> PyResult<Bound<'py, PyDict>> {
        node_to_dict(py, &self.inner.node(object_id).map_err(to_py_err)?)
    }
}

/// The Mnemosyne core version.
#[pyfunction]
fn version() -> &'static str {
    mnem_core::version()
}

#[pymodule]
fn _mnem(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", mnem_core::version())?;
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_class::<PyStore>()?;

    m.add("MnemError", m.py().get_type::<MnemErrorException>())?;
    m.add("NotFoundError", m.py().get_type::<NotFoundError>())?;
    m.add("InvalidRefError", m.py().get_type::<InvalidRefError>())?;
    m.add("ConflictError", m.py().get_type::<ConflictError>())?;
    m.add("CorruptStoreError", m.py().get_type::<CorruptStoreError>())?;
    m.add(
        "FormatVersionError",
        m.py().get_type::<FormatVersionError>(),
    )?;
    m.add("StoreExistsError", m.py().get_type::<StoreExistsError>())?;
    m.add("NoStoreError", m.py().get_type::<NoStoreError>())?;
    m.add("StoreIoError", m.py().get_type::<StoreIoError>())?;
    Ok(())
}
