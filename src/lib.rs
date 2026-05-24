//! UUIDv7 newtype with short ID display and efficient serialization.
//!
//! Provides [`Dandruff`], a wrapper around [`Uuid`] that displays as a short 8-character hex
//! suffix by default, while preserving all the benefits of UUIDv7: time-ordered, globally unique,
//! and database-friendly.

use std::{fmt, str::FromStr};

use thiserror::Error;
use uuid::Uuid;

/// A UUIDv7 identifier with convenient short display.
///
/// Displays as the last 8 hex characters by default (e.g., `a1b2c3d4`), which provides a
/// human-friendly identifier while maintaining uniqueness in most practical scenarios. Use the
/// alternate format `{:#}` for the full hyphenated UUID.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Dandruff(Uuid);

impl Dandruff {
    /// Generates a new UUIDv7-based identifier.
    ///
    /// Uses the current timestamp and random bits to create a time-ordered, globally unique ID.
    #[inline]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Wraps an existing UUID without validating that it is version 7.
    ///
    /// Semantic guarantees of [`Dandruff`] (time ordering, timestamp extraction) only hold for
    /// actual v7 UUIDs. Use [`TryFrom`] for validated conversion.
    #[inline]
    pub fn from_uuid_unchecked(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns a reference to the underlying UUID.
    #[inline]
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Consumes self and returns the underlying UUID.
    #[inline]
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for Dandruff {
    /// Generates a new UUIDv7-based identifier.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Dandruff {
    /// Formats as the short ID (last 8 hex characters).
    ///
    /// Use `{:#}` for the full hyphenated UUID format.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.0.as_hyphenated())
        } else {
            let buf = &mut [0u8; 32];
            let s = self.0.as_simple().encode_lower(buf);
            f.write_str(&s[24..])
        }
    }
}

impl FromStr for Dandruff {
    type Err = ParseError;

    /// Parses a full UUID string into a [`Dandruff`].
    ///
    /// Only accepts complete UUID strings (hyphenated or simple format). For resolving short IDs
    /// against a collection, use [`resolve`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(s).map_err(ParseError::Uuid)?;
        Self::try_from(uuid).map_err(ParseError::NotV7)
    }
}

impl TryFrom<Uuid> for Dandruff {
    type Error = NotV7Error;

    /// Converts a UUID to a [`Dandruff`], validating that it is version 7.
    fn try_from(uuid: Uuid) -> Result<Self, Self::Error> {
        if uuid.get_version_num() == 7 {
            Ok(Self(uuid))
        } else {
            Err(NotV7Error {
                version: uuid.get_version_num(),
            })
        }
    }
}

impl AsRef<Uuid> for Dandruff {
    /// Returns a reference to the underlying UUID.
    #[inline]
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl AsRef<[u8; 16]> for Dandruff {
    /// Returns a reference to the underlying bytes.
    #[inline]
    fn as_ref(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl From<Dandruff> for Uuid {
    /// Converts to the underlying UUID.
    #[inline]
    fn from(d: Dandruff) -> Self {
        d.0
    }
}

/// Error returned when a UUID is not version 7.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("expected UUID version 7, got version {version}")]
pub struct NotV7Error {
    /// The actual version number of the UUID.
    pub version: usize,
}

/// Error returned when parsing a string as a [`Dandruff`].
#[derive(Debug, Error)]
pub enum ParseError {
    /// The string is not a valid UUID.
    #[error("invalid UUID: {0}")]
    Uuid(#[source] uuid::Error),

    /// The UUID is valid but not version 7.
    #[error("{0}")]
    NotV7(#[source] NotV7Error),
}

/// Error returned when resolving a short ID.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ResolveError {
    /// The pattern is not a valid hex string or has invalid length.
    #[error("invalid pattern: must be 4-32 hex characters")]
    InvalidPattern,

    /// No ID matched the pattern.
    #[error("no ID matched the pattern")]
    NotFound,

    /// Multiple IDs matched the pattern.
    #[error("pattern is ambiguous, matched {} IDs", .0.len())]
    Ambiguous(Vec<Dandruff>),
}

/// Resolves a short ID pattern against a collection of IDs.
///
/// Accepts 4-32 hex character patterns. Input is normalized: dashes and spaces are stripped,
/// letters are lowercased. The pattern matches as a substring anywhere in the UUID's hex
/// representation, though typically the last 8 characters (the short ID) are used.
///
/// # Errors
///
/// Returns [`ResolveError::InvalidPattern`] if the pattern is not valid hex or has invalid length.
/// Returns [`ResolveError::NotFound`] if no ID matches.
/// Returns [`ResolveError::Ambiguous`] if multiple IDs match, containing all matches.
pub fn resolve<'a, I>(iter: I, pattern: &str) -> Result<Dandruff, ResolveError>
where
    I: IntoIterator<Item = &'a Dandruff>,
{
    let normalized: String = pattern
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(|c| c.to_lowercase())
        .collect();

    if normalized.len() < 4 || normalized.len() > 32 {
        return Err(ResolveError::InvalidPattern);
    }

    if !normalized.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ResolveError::InvalidPattern);
    }

    let matches: Vec<Dandruff> = iter
        .into_iter()
        .filter(|id| {
            let hex = id.0.as_simple().to_string();
            hex.contains(&normalized)
        })
        .copied()
        .collect();

    match matches.len() {
        0 => Err(ResolveError::NotFound),
        1 => Ok(matches[0]),
        _ => Err(ResolveError::Ambiguous(matches)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_v7() {
        let id = Dandruff::new();
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn display_shows_short_id() {
        let id = Dandruff::new();
        let short = format!("{}", id);
        assert_eq!(short.len(), 8);
        assert!(short.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn alternate_display_shows_full_uuid() {
        let id = Dandruff::new();
        let full = format!("{:#}", id);
        assert_eq!(full.len(), 36);
        assert!(full.contains('-'));
    }

    #[test]
    fn from_str_roundtrip() {
        let id = Dandruff::new();
        let s = format!("{:#}", id);
        let parsed: Dandruff = s.parse().expect("should parse");
        assert_eq!(id, parsed);
    }

    #[test]
    fn try_from_rejects_non_v7() {
        let nil = Uuid::nil();
        let result = Dandruff::try_from(nil);
        assert!(matches!(result, Err(NotV7Error { version: 0 })));
    }

    #[test]
    fn from_uuid_unchecked_accepts_any() {
        let nil = Uuid::nil();
        let id = Dandruff::from_uuid_unchecked(nil);
        assert_eq!(id.as_uuid().get_version_num(), 0);
    }

    #[test]
    fn resolve_finds_unique_match() {
        let id1 = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("01234567-89ab-7def-8000-000011111111").expect("valid uuid"),
        );
        let id2 = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("01234567-89ab-7def-8000-000022222222").expect("valid uuid"),
        );
        let ids = vec![id1, id2];

        assert_eq!(resolve(&ids, "11111111"), Ok(id1));
        assert_eq!(resolve(&ids, "22222222"), Ok(id2));
    }

    #[test]
    fn resolve_returns_not_found() {
        let id = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("01234567-89ab-7def-8000-000011111111").expect("valid uuid"),
        );
        let ids = vec![id];

        let result = resolve(&ids, "deadbeef");
        assert!(matches!(result, Err(ResolveError::NotFound)));
    }

    #[test]
    fn resolve_returns_ambiguous() {
        let id1 = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("01234567-89ab-7def-8000-000012345678").expect("valid uuid"),
        );
        let id2 = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("fedcba98-7654-7321-8000-000012345678").expect("valid uuid"),
        );
        let ids = vec![id1, id2];

        let result = resolve(&ids, "12345678");
        assert!(matches!(result, Err(ResolveError::Ambiguous(ref v)) if v.len() == 2));
    }

    #[test]
    fn resolve_rejects_invalid_pattern() {
        let id = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("01234567-89ab-7def-8000-000011111111").expect("valid uuid"),
        );
        let ids = vec![id];

        assert!(matches!(
            resolve(&ids, "abc"),
            Err(ResolveError::InvalidPattern)
        ));
        assert!(matches!(
            resolve(&ids, "ghij1234"),
            Err(ResolveError::InvalidPattern)
        ));
    }

    #[test]
    fn resolve_normalizes_input() {
        let id = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("01234567-89ab-7def-8000-aabbccddeeff").expect("valid uuid"),
        );
        let ids = vec![id];

        assert_eq!(resolve(&ids, "AABBCCDDEEFF"), Ok(id));
        assert_eq!(resolve(&ids, "aabb-ccdd"), Ok(id));
        assert_eq!(resolve(&ids, "AABB CCDD"), Ok(id));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn display_fromstr_roundtrip(_seed: u64) {
            let id = Dandruff::new();
            let full = format!("{:#}", id);
            let parsed: Dandruff = full.parse().expect("roundtrip should succeed");
            prop_assert_eq!(id, parsed);
        }
    }
}
