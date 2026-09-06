//! [`ObjectId`], the identity of every object in a store.
//!
//! An id is the BLAKE3 hash of an object's canonical bytes (ADR-0005). It is
//! 32 bytes, shown as 64 lowercase hex characters, and serialises as a CBOR
//! byte string rather than an array of integers.

use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A 32-byte BLAKE3 content hash. The id of a memory node, a state, or a commit.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId([u8; 32]);

impl ObjectId {
    /// The number of bytes in an id.
    pub const LEN: usize = 32;

    /// Wrap 32 raw bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The id of `bytes`, which the caller must have produced by the canonical
    /// encoder (ADR-0008). Hashing non-canonical bytes gives a meaningless id.
    pub fn hash_canonical(bytes: &[u8]) -> Self {
        Self(blake3::hash(bytes).into())
    }

    /// The 64-character lowercase hex form.
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for byte in self.0 {
            s.push(char::from_digit((byte >> 4) as u32, 16).unwrap());
            s.push(char::from_digit((byte & 0x0f) as u32, 16).unwrap());
        }
        s
    }
}

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectId({})", self.to_hex())
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// The error from parsing an [`ObjectId`] from hex.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseIdError {
    /// The string was not exactly 64 characters.
    WrongLength(usize),
    /// The string held a character that is not a hex digit.
    NotHex,
}

impl fmt::Display for ParseIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongLength(n) => write!(f, "expected 64 hex characters, got {n}"),
            Self::NotHex => f.write_str("expected only hex characters"),
        }
    }
}

impl std::error::Error for ParseIdError {}

impl FromStr for ObjectId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 64 {
            return Err(ParseIdError::WrongLength(s.len()));
        }
        let mut bytes = [0u8; 32];
        let raw = s.as_bytes();
        for (i, byte) in bytes.iter_mut().enumerate() {
            let hi = (raw[i * 2] as char).to_digit(16).ok_or(ParseIdError::NotHex)?;
            let lo = (raw[i * 2 + 1] as char)
                .to_digit(16)
                .ok_or(ParseIdError::NotHex)?;
            *byte = ((hi << 4) | lo) as u8;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for ObjectId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IdVisitor;

        impl<'de> Visitor<'de> for IdVisitor {
            type Value = ObjectId;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a 32-byte object id")
            }

            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<ObjectId, E> {
                let bytes: [u8; 32] = v
                    .try_into()
                    .map_err(|_| E::invalid_length(v.len(), &"32 bytes"))?;
                Ok(ObjectId(bytes))
            }

            fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<ObjectId, E> {
                self.visit_bytes(&v)
            }
        }

        deserializer.deserialize_bytes(IdVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        let id = ObjectId::hash_canonical(b"hello");
        let hex = id.to_hex();
        assert_eq!(hex.len(), 64);
        assert_eq!(hex.parse::<ObjectId>().unwrap(), id);
    }

    #[test]
    fn hex_is_stable() {
        // BLAKE3 of the empty input is a fixed value.
        let id = ObjectId::hash_canonical(b"");
        assert_eq!(
            id.to_hex(),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn parse_rejects_bad_input() {
        assert_eq!("abc".parse::<ObjectId>(), Err(ParseIdError::WrongLength(3)));
        let sixty_four_zs = "z".repeat(64);
        assert_eq!(sixty_four_zs.parse::<ObjectId>(), Err(ParseIdError::NotHex));
    }
}
