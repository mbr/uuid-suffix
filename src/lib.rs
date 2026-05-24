#![doc = include_str!("../README.md")]

use std::{fmt, str::FromStr};

use thiserror::Error;
use uuid::Uuid;

/// A parsed tail ID for efficient suffix matching against UUIDs.
///
/// Stores the tail ID as a `u128` value with a length field, enabling fast bitwise comparison.
/// Accepts 1-32 hex characters (dashes and spaces are stripped during parsing, case-insensitive).
///
/// # Example
///
/// ```
/// use tail_id::TailId;
///
/// let tail: TailId = "3f6a4e7".parse().unwrap();
/// assert_eq!(format!("{}", tail), "3f6a4e7");
/// ```
#[allow(clippy::len_without_is_empty)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TailId {
    /// The tail ID value, right-aligned (least significant bits).
    value: u128,
    /// Number of hex digits (1-32).
    len: u8,
}

impl TailId {
    /// Minimum number of hex characters required.
    pub const MIN_LEN: u8 = 1;
    /// Maximum number of hex characters allowed (full UUID).
    pub const MAX_LEN: u8 = 32;
    /// Standard length for tail IDs (7 hex chars = 28 bits).
    pub const STANDARD_LEN: u8 = 7;

    /// Creates a tail ID from a UUID with the standard length (7 hex chars).
    #[inline]
    pub fn new(uuid: &Uuid) -> Self {
        Self::with_len(uuid, Self::STANDARD_LEN)
    }

    /// Creates a tail ID from a UUID with the specified length.
    ///
    /// # Panics
    ///
    /// Panics if `len` is 0 or greater than 32.
    #[inline]
    pub fn with_len(uuid: &Uuid, len: u8) -> Self {
        assert!((Self::MIN_LEN..=Self::MAX_LEN).contains(&len));
        let mask = if len == 32 {
            u128::MAX
        } else {
            (1u128 << (len as u32 * 4)) - 1
        };
        TailId {
            value: uuid.as_u128() & mask,
            len,
        }
    }

    /// Returns the number of hex digits in this tail ID.
    #[inline]
    pub fn len(&self) -> u8 {
        self.len
    }

    /// Checks if this tail ID matches the suffix of the given UUID.
    #[inline]
    pub fn matches(&self, uuid: &Uuid) -> bool {
        let mask = if self.len == 32 {
            u128::MAX
        } else {
            (1u128 << (self.len as u32 * 4)) - 1
        };
        // Note: `uuid::as_u128` packs the rightmost bytes of the UUID into the LSBs,
        //       thus "read big-endian", which is what we need.
        (uuid.as_u128() & mask) == self.value
    }
}

impl fmt::Display for TailId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:0>width$x}", self.value, width = self.len as usize)
    }
}

impl FromStr for TailId {
    type Err = ParseError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<&[u8]> for TailId {
    type Error = ParseError;

    #[inline]
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut buf = [0u8; 32];
        let mut len = 0usize;

        for &b in bytes {
            if b == b'-' || b == b' ' {
                continue;
            }
            if len >= 32 {
                return Err(ParseError::TooLong);
            }
            if !b.is_ascii_hexdigit() {
                return Err(ParseError::InvalidByte(b));
            }
            buf[len] = b.to_ascii_lowercase();
            len += 1;
        }

        if len == 0 {
            return Err(ParseError::Empty);
        }

        // SAFETY: buf contains only ASCII hex digits, which are valid UTF-8.
        let s = unsafe { std::str::from_utf8_unchecked(&buf[..len]) };
        let value =
            u128::from_str_radix(s, 16).expect("input validated as hex digits, cannot fail");

        Ok(TailId {
            value,
            len: len as u8,
        })
    }
}

impl TryFrom<&str> for TailId {
    type Error = ParseError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::try_from(s.as_bytes())
    }
}

/// Error returned when parsing a [`TailId`].
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ParseError {
    /// The input is empty after stripping dashes and spaces.
    #[error("tail ID cannot be empty")]
    Empty,

    /// The input exceeds 32 hex characters.
    #[error("tail ID cannot exceed 32 hex characters")]
    TooLong,

    /// The input contains a non-hex byte.
    #[error("invalid byte in tail ID: 0x{0:02x}")]
    InvalidByte(u8),
}

/// Error returned when resolving a tail ID.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ResolveError {
    /// No UUID matched the pattern.
    #[error("no UUID matched the pattern")]
    NotFound,

    /// Multiple UUIDs matched the pattern.
    #[error("pattern is ambiguous, matched {} UUIDs", .0.len())]
    Ambiguous(Vec<Uuid>),
}

/// Resolves a [`TailId`] against a collection of UUIDs.
///
/// Returns the unique matching UUID, or an error if zero or multiple UUIDs match.
pub fn resolve_tail_id<'a, I>(iter: I, tail_id: &TailId) -> Result<Uuid, ResolveError>
where
    I: IntoIterator<Item = &'a Uuid>,
{
    let mut iter = iter.into_iter().filter(|id| tail_id.matches(id));

    let first = *iter.next().ok_or(ResolveError::NotFound)?;

    let Some(&second) = iter.next() else {
        return Ok(first);
    };

    let mut matches = vec![first, second];
    matches.extend(iter.copied());
    Err(ResolveError::Ambiguous(matches))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid() {
        let tail: TailId = "3f6a4e7".parse().expect("valid");
        assert_eq!(tail.len(), 7);
    }

    #[test]
    fn parse_normalizes() {
        let lower: TailId = "abcd".parse().expect("valid");
        let upper: TailId = "ABCD".parse().expect("valid");
        let dashes: TailId = "ab-cd".parse().expect("valid");
        let spaces: TailId = "ab cd".parse().expect("valid");

        assert_eq!(lower, upper);
        assert_eq!(lower, dashes);
        assert_eq!(lower, spaces);
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(matches!(TailId::try_from(""), Err(ParseError::Empty)));
        assert!(matches!(TailId::try_from("  - - "), Err(ParseError::Empty)));
    }

    #[test]
    fn parse_rejects_too_long() {
        let result = TailId::try_from("0123456789abcdef0123456789abcdef0");
        assert!(matches!(result, Err(ParseError::TooLong)));
    }

    #[test]
    fn parse_rejects_invalid_bytes() {
        assert!(matches!(
            TailId::try_from("ghij"),
            Err(ParseError::InvalidByte(b'g'))
        ));
    }

    #[test]
    fn display_roundtrip() {
        let original = "3f6a4e7";
        let tail: TailId = original.parse().expect("valid");
        assert_eq!(format!("{}", tail), original);
    }

    #[test]
    fn display_preserves_leading_zeros() {
        let tail: TailId = "00abcd".parse().expect("valid");
        assert_eq!(format!("{}", tail), "00abcd");
    }

    #[test]
    fn matches_suffix() {
        let uuid = Uuid::parse_str("01234567-89ab-7def-8000-aabbccddeeff").expect("valid");

        let suffix: TailId = "eeff".parse().expect("valid");
        assert!(suffix.matches(&uuid));

        let full: TailId = "0123456789ab7def8000aabbccddeeff".parse().expect("valid");
        assert!(full.matches(&uuid));

        let wrong: TailId = "ffff".parse().expect("valid");
        assert!(!wrong.matches(&uuid));
    }

    #[test]
    fn from_bytes() {
        let tail = TailId::try_from(b"3f6a4e7".as_slice()).expect("valid");
        assert_eq!(tail.len(), 7);
        assert_eq!(format!("{}", tail), "3f6a4e7");
    }

    #[test]
    fn resolve_finds_unique_match() {
        let id1 = Uuid::parse_str("01234567-89ab-7def-8000-000011111111").expect("valid");
        let id2 = Uuid::parse_str("01234567-89ab-7def-8000-000022222222").expect("valid");
        let ids = vec![id1, id2];

        let tail1: TailId = "11111111".parse().expect("valid");
        let tail2: TailId = "22222222".parse().expect("valid");
        assert_eq!(resolve_tail_id(&ids, &tail1), Ok(id1));
        assert_eq!(resolve_tail_id(&ids, &tail2), Ok(id2));
    }

    #[test]
    fn resolve_returns_not_found() {
        let id = Uuid::parse_str("01234567-89ab-7def-8000-000011111111").expect("valid");
        let ids = vec![id];

        let tail: TailId = "deadbeef".parse().expect("valid");
        assert!(matches!(
            resolve_tail_id(&ids, &tail),
            Err(ResolveError::NotFound)
        ));
    }

    #[test]
    fn resolve_returns_ambiguous() {
        let id1 = Uuid::parse_str("01234567-89ab-7def-8000-000012345678").expect("valid");
        let id2 = Uuid::parse_str("fedcba98-7654-7321-8000-000012345678").expect("valid");
        let ids = vec![id1, id2];

        let tail: TailId = "12345678".parse().expect("valid");
        let result = resolve_tail_id(&ids, &tail);
        assert!(matches!(result, Err(ResolveError::Ambiguous(ref v)) if v.len() == 2));
    }
}
