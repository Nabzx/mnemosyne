//! `HEAD`, the current pointer (ADR-0009).
//!
//! `HEAD` is a one-line plain-text file. Attached, it names a branch and moves
//! with it. Detached, it points straight at a commit. A fresh store is attached
//! to a branch that does not exist yet.

use crate::error::{MnemError, Result};
use crate::id::ObjectId;
use crate::refs;

/// Where `HEAD` points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    /// Follows a branch. The branch need not exist.
    Attached(String),
    /// Points straight at a commit.
    Detached(ObjectId),
}

impl Head {
    /// Render the one-line file contents, newline included.
    pub fn to_file_string(&self) -> String {
        match self {
            Head::Attached(name) => format!("ref: {name}\n"),
            Head::Detached(id) => format!("{id}\n"),
        }
    }

    /// Parse the file contents.
    pub fn parse(text: &str) -> Result<Self> {
        let line = text.trim();
        if let Some(rest) = line.strip_prefix("ref:") {
            let name = rest.trim();
            if name.is_empty() {
                return Err(MnemError::CorruptStore(
                    "HEAD: 'ref:' with no branch name".to_string(),
                ));
            }
            refs::validate_name(name)?;
            Ok(Head::Attached(name.to_string()))
        } else {
            line.parse::<ObjectId>().map(Head::Detached).map_err(|e| {
                MnemError::CorruptStore(format!("HEAD is not a ref or a commit id: {e}"))
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attached_round_trips() {
        let head = Head::Attached("main".to_string());
        assert_eq!(head.to_file_string(), "ref: main\n");
        assert_eq!(Head::parse(&head.to_file_string()).unwrap(), head);
    }

    #[test]
    fn detached_round_trips() {
        let head = Head::Detached(ObjectId::hash_canonical(b"a commit"));
        assert_eq!(Head::parse(&head.to_file_string()).unwrap(), head);
    }

    #[test]
    fn rejects_junk() {
        assert!(matches!(
            Head::parse("not a thing\n"),
            Err(MnemError::CorruptStore(_))
        ));
        assert!(matches!(
            Head::parse("ref:\n"),
            Err(MnemError::CorruptStore(_))
        ));
        assert!(Head::parse("ref: HEAD\n").is_err());
    }
}
