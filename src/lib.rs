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

    /// Returns the number of hex digits in this tail ID.
    #[inline]
    pub fn len(&self) -> u8 {
        self.len
    }

    /// Returns `true` if this tail ID has zero length.
    ///
    /// Note: A zero-length TailId cannot be constructed via parsing, so this always returns
    /// `false` for validly constructed instances.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the underlying value as a `u128`.
    ///
    /// The value is right-aligned: for a 7-character tail ID "3f6a4e7", the value is `0x3f6a4e7`.
    #[inline]
    pub fn as_u128(&self) -> u128 {
        self.value
    }

    /// Checks if this tail ID matches the suffix of the given UUID.
    #[inline]
    pub fn matches(&self, uuid: &Uuid) -> bool {
        let mask = if self.len == 32 {
            u128::MAX
        } else {
            (1u128 << (self.len as u32 * 4)) - 1
        };
        (uuid.as_u128() & mask) == self.value
    }
}

impl fmt::Display for TailId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.len == 0 {
            return Ok(());
        }
        write!(f, "{:0>width$x}", self.value, width = self.len as usize)
    }
}

impl FromStr for TailId {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<&str> for TailId {
    type Error = ParseError;

    /// Parses a tail ID from a string.
    ///
    /// Strips dashes and whitespace, normalizes to lowercase. Accepts 1-32 hex characters.
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut normalized = [0u8; 32];
        let mut len = 0usize;

        for c in s.chars() {
            if c == '-' || c.is_whitespace() {
                continue;
            }
            if len >= 32 {
                return Err(ParseError::TooLong);
            }
            let hex_digit = match c {
                '0'..='9' => c as u8 - b'0',
                'a'..='f' => c as u8 - b'a' + 10,
                'A'..='F' => c as u8 - b'A' + 10,
                _ => return Err(ParseError::InvalidCharacter(c)),
            };
            normalized[len] = hex_digit;
            len += 1;
        }

        if len == 0 {
            return Err(ParseError::Empty);
        }

        let mut value = 0u128;
        for &digit in &normalized[..len] {
            value = (value << 4) | (digit as u128);
        }

        Ok(TailId {
            value,
            len: len as u8,
        })
    }
}

impl TryFrom<&[u8]> for TailId {
    type Error = ParseError;

    /// Parses a tail ID from a byte slice.
    ///
    /// Strips dashes and whitespace, normalizes to lowercase. Accepts 1-32 hex characters.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut normalized = [0u8; 32];
        let mut len = 0usize;

        for &b in bytes {
            if b == b'-' || b.is_ascii_whitespace() {
                continue;
            }
            if len >= 32 {
                return Err(ParseError::TooLong);
            }
            let hex_digit = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(ParseError::InvalidCharacter(b as char)),
            };
            normalized[len] = hex_digit;
            len += 1;
        }

        if len == 0 {
            return Err(ParseError::Empty);
        }

        let mut value = 0u128;
        for &digit in &normalized[..len] {
            value = (value << 4) | (digit as u128);
        }

        Ok(TailId {
            value,
            len: len as u8,
        })
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

    /// The input contains a non-hex character.
    #[error("invalid character in tail ID: '{0}'")]
    InvalidCharacter(char),
}

/// Error returned when resolving a tail ID.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ResolveError {
    /// The pattern could not be parsed as a [`TailId`].
    #[error("invalid tail ID: {0}")]
    InvalidTailId(#[source] ParseError),

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
    let matches: Vec<Uuid> = iter
        .into_iter()
        .filter(|id| tail_id.matches(id))
        .copied()
        .collect();

    match matches.len() {
        0 => Err(ResolveError::NotFound),
        1 => Ok(matches[0]),
        _ => Err(ResolveError::Ambiguous(matches)),
    }
}

/// Resolves a tail ID pattern against a collection of UUIDs.
///
/// Parses the pattern as a [`TailId`] and returns the unique matching UUID.
pub fn resolve<'a, I>(iter: I, pattern: &str) -> Result<Uuid, ResolveError>
where
    I: IntoIterator<Item = &'a Uuid>,
{
    let tail_id = TailId::try_from(pattern).map_err(ResolveError::InvalidTailId)?;
    resolve_tail_id(iter, &tail_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid() {
        let tail: TailId = "3f6a4e7".parse().expect("valid");
        assert_eq!(tail.len(), 7);
        assert_eq!(tail.as_u128(), 0x3f6a4e7);
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
    fn parse_rejects_invalid_chars() {
        assert!(matches!(
            TailId::try_from("ghij"),
            Err(ParseError::InvalidCharacter('g'))
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

        assert_eq!(resolve(&ids, "11111111"), Ok(id1));
        assert_eq!(resolve(&ids, "22222222"), Ok(id2));
    }

    #[test]
    fn resolve_returns_not_found() {
        let id = Uuid::parse_str("01234567-89ab-7def-8000-000011111111").expect("valid");
        let ids = vec![id];

        assert!(matches!(
            resolve(&ids, "deadbeef"),
            Err(ResolveError::NotFound)
        ));
    }

    #[test]
    fn resolve_returns_ambiguous() {
        let id1 = Uuid::parse_str("01234567-89ab-7def-8000-000012345678").expect("valid");
        let id2 = Uuid::parse_str("fedcba98-7654-7321-8000-000012345678").expect("valid");
        let ids = vec![id1, id2];

        let result = resolve(&ids, "12345678");
        assert!(matches!(result, Err(ResolveError::Ambiguous(ref v)) if v.len() == 2));
    }

    #[test]
    fn resolve_normalizes_input() {
        let id = Uuid::parse_str("01234567-89ab-7def-8000-aabbccddeeff").expect("valid");
        let ids = vec![id];

        assert_eq!(resolve(&ids, "AABBCCDDEEFF"), Ok(id));
        assert_eq!(resolve(&ids, "CCDD-EEFF"), Ok(id));
        assert_eq!(resolve(&ids, "ddee ff"), Ok(id));
    }
}
