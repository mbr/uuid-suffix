#![doc = include_str!("../README.md")]

use std::{fmt, str::FromStr};

use thiserror::Error;
use uuid::Uuid;

/// A time-ordered UUID (v6 or v7) with convenient short display.
///
/// Displays as the last 7 hex characters by default. Use width specifier for more (e.g.,
/// `{:9}`), or `{:#}` for the full hyphenated UUID.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Dandruff(Uuid);

impl Dandruff {
    /// Generates a new UUIDv7-based identifier.
    ///
    /// Uses the current timestamp and random bits to create a time-ordered, globally unique ID.
    #[cfg(feature = "v7")]
    #[inline]
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    /// Generates a new UUIDv6-based identifier with the given node ID.
    ///
    /// Uses the current timestamp and the provided 6-byte node ID. This is useful when you need
    /// deterministic IDs based on machine identity, similar to Snowflake IDs.
    #[cfg(feature = "v6")]
    #[inline]
    pub fn new_v6(node_id: &[u8; 6]) -> Self {
        Self(Uuid::now_v6(node_id))
    }

    /// Wraps an existing UUID without validating its version.
    ///
    /// Semantic guarantees of [`Dandruff`] (time ordering, timestamp extraction) only hold for
    /// v6 or v7 UUIDs. Use [`TryFrom`] for validated conversion.
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

#[cfg(feature = "v7")]
impl Default for Dandruff {
    /// Generates a new UUIDv7-based identifier.
    #[inline]
    fn default() -> Self {
        Self::new_v7()
    }
}

/// Default short ID length (7 hex characters).
const DEFAULT_SHORT_LEN: usize = 7;

impl fmt::Display for Dandruff {
    /// Formats as the short ID (last 7 hex characters by default).
    ///
    /// Use width specifier for longer IDs (e.g., `{:9}` for 9 chars).
    /// Use `{:#}` for the full hyphenated UUID format.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.0.as_hyphenated())
        } else {
            let width = f.width().unwrap_or(DEFAULT_SHORT_LEN).min(32);
            let buf = &mut [0u8; 32];
            let s = self.0.as_simple().encode_lower(buf);
            f.write_str(&s[32 - width..])
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
        Self::try_from(uuid).map_err(ParseError::Version)
    }
}

impl TryFrom<Uuid> for Dandruff {
    type Error = VersionError;

    /// Converts a UUID to a [`Dandruff`], validating that it is version 6 or 7.
    fn try_from(uuid: Uuid) -> Result<Self, Self::Error> {
        let version = uuid.get_version_num();
        if version == 6 || version == 7 {
            Ok(Self(uuid))
        } else {
            Err(VersionError { version })
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

/// Error returned when a UUID is not a time-ordered version (v6 or v7).
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("expected UUID version 6 or 7, got version {version}")]
pub struct VersionError {
    /// The actual version number of the UUID.
    pub version: usize,
}

/// Error returned when parsing a string as a [`Dandruff`].
#[derive(Debug, Error)]
pub enum ParseError {
    /// The string is not a valid UUID.
    #[error("invalid UUID: {0}")]
    Uuid(#[source] uuid::Error),

    /// The UUID is valid but not a time-ordered version.
    #[error("{0}")]
    Version(#[source] VersionError),
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
/// representation, though typically the last 7 characters (the short ID) are used.
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

    #[cfg(feature = "v7")]
    #[test]
    fn new_creates_v7() {
        let id = Dandruff::new_v7();
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[cfg(feature = "v6")]
    #[test]
    fn new_v6_creates_v6() {
        let node_id = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let id = Dandruff::new_v6(&node_id);
        assert_eq!(id.as_uuid().get_version_num(), 6);
    }

    #[cfg(feature = "v6")]
    #[test]
    fn try_from_accepts_v6() {
        let node_id = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let uuid = Uuid::now_v6(&node_id);
        let result = Dandruff::try_from(uuid);
        assert!(result.is_ok());
        assert_eq!(result.expect("should be ok").as_uuid().get_version_num(), 6);
    }

    #[cfg(feature = "v7")]
    #[test]
    fn display_shows_short_id() {
        let id = Dandruff::new_v7();
        let short = format!("{}", id);
        assert_eq!(short.len(), 7);
        assert!(short.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[cfg(feature = "v7")]
    #[test]
    fn display_respects_width() {
        let id = Dandruff::new_v7();
        assert_eq!(format!("{:9}", id).len(), 9);
        assert_eq!(format!("{:12}", id).len(), 12);
        assert_eq!(format!("{:32}", id).len(), 32);
    }

    #[cfg(feature = "v7")]
    #[test]
    fn alternate_display_shows_full_uuid() {
        let id = Dandruff::new_v7();
        let full = format!("{:#}", id);
        assert_eq!(full.len(), 36);
        assert!(full.contains('-'));
    }

    #[cfg(feature = "v7")]
    #[test]
    fn from_str_roundtrip() {
        let id = Dandruff::new_v7();
        let s = format!("{:#}", id);
        let parsed: Dandruff = s.parse().expect("should parse");
        assert_eq!(id, parsed);
    }

    #[test]
    fn try_from_rejects_non_time_ordered() {
        let nil = Uuid::nil();
        let result = Dandruff::try_from(nil);
        assert!(matches!(result, Err(VersionError { version: 0 })));
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

#[cfg(all(test, feature = "v7"))]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn display_fromstr_roundtrip(_seed: u64) {
            let id = Dandruff::new_v7();
            let full = format!("{:#}", id);
            let parsed: Dandruff = full.parse().expect("roundtrip should succeed");
            prop_assert_eq!(id, parsed);
        }
    }
}
