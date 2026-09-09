//! The `.mnem/config` file (ADR-0002, ADR-0005, ADR-0007, ADR-0009).
//!
//! A short plain-text file of `key = value` lines, so a person can read or set
//! it by hand. Unknown keys are ignored, which lets a newer store add a key
//! without breaking an older reader; the `format_version` gate is what stops an
//! older reader from misusing a store it does not understand.

use crate::error::{MnemError, Result};

/// The highest `format_version` this release can read and write.
pub const SUPPORTED_FORMAT_VERSION: u32 = 1;

/// The parsed contents of `.mnem/config`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The on-disk format version. Its own track, not the software version
    /// (ADR-0007).
    pub format_version: u32,
    /// The content hash algorithm. Always `blake3` in this release (ADR-0005).
    pub hash_algo: String,
    /// The branch a fresh store's `HEAD` points at (ADR-0009).
    pub default_branch: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            format_version: SUPPORTED_FORMAT_VERSION,
            hash_algo: "blake3".to_string(),
            default_branch: "main".to_string(),
        }
    }
}

impl Config {
    /// Render the config file, including the header comment.
    pub fn to_file_string(&self) -> String {
        format!(
            "# Mnemosyne store config. See docs/format/.\n\
             format_version = {}\n\
             hash_algo = {}\n\
             default_branch = {}\n",
            self.format_version, self.hash_algo, self.default_branch
        )
    }

    /// Parse a config file. Missing required keys, or a `format_version` this
    /// release cannot read, are errors.
    pub fn parse(text: &str) -> Result<Self> {
        let mut format_version: Option<u32> = None;
        let mut hash_algo: Option<String> = None;
        let mut default_branch: Option<String> = None;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(MnemError::CorruptStore(format!(
                    "config line has no `=`: {line}"
                )));
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "format_version" => {
                    format_version = Some(value.parse().map_err(|_| {
                        MnemError::CorruptStore(format!("format_version is not a number: {value}"))
                    })?);
                }
                "hash_algo" => hash_algo = Some(value.to_string()),
                "default_branch" => default_branch = Some(value.to_string()),
                _ => {} // ignore unknown keys
            }
        }

        let format_version = format_version
            .ok_or_else(|| MnemError::CorruptStore("config has no format_version".to_string()))?;
        if format_version > SUPPORTED_FORMAT_VERSION {
            return Err(MnemError::FormatVersion {
                found: format_version,
                supported: SUPPORTED_FORMAT_VERSION,
            });
        }

        let hash_algo = hash_algo
            .ok_or_else(|| MnemError::CorruptStore("config has no hash_algo".to_string()))?;
        if hash_algo != "blake3" {
            return Err(MnemError::CorruptStore(format!(
                "unsupported hash_algo: {hash_algo}"
            )));
        }

        Ok(Self {
            format_version,
            hash_algo,
            default_branch: default_branch.unwrap_or_else(|| "main".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let config = Config::default();
        assert_eq!(Config::parse(&config.to_file_string()).unwrap(), config);
    }

    #[test]
    fn ignores_unknown_keys_and_comments() {
        let text = "# a note\nformat_version = 1\nhash_algo = blake3\nsomething_new = 42\n";
        assert_eq!(Config::parse(text).unwrap(), Config::default());
    }

    #[test]
    fn rejects_a_future_format_version() {
        let text = "format_version = 99\nhash_algo = blake3\n";
        assert!(matches!(
            Config::parse(text),
            Err(MnemError::FormatVersion { found: 99, .. })
        ));
    }

    #[test]
    fn rejects_a_missing_required_key() {
        assert!(matches!(
            Config::parse("hash_algo = blake3\n"),
            Err(MnemError::CorruptStore(_))
        ));
    }

    #[test]
    fn rejects_an_unknown_hash_algo() {
        assert!(matches!(
            Config::parse("format_version = 1\nhash_algo = md5\n"),
            Err(MnemError::CorruptStore(_))
        ));
    }
}
