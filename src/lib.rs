#![doc = include_str!("../README.md")]

use std::{fmt, str::FromStr};

#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;
use uuid::Uuid;

/// A parsed tail ID for efficient suffix matching against UUIDs.
///
/// Stores the tail ID as a `u128` value with a length field, enabling fast bitwise comparison.
/// Accepts 1 to 32 hex characters (dashes are stripped during parsing, case-insensitive).
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
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct TailId {
    /// Number of hex digits (MIN_LEN to MAX_LEN).
    len: u8,
    /// The tail ID value, right-aligned (least significant bits).
    value: u128,
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
    /// Panics if `len` is outside `MIN_LEN..=MAX_LEN`.
    #[inline]
    pub fn with_len(uuid: &Uuid, len: u8) -> Self {
        assert!((Self::MIN_LEN..=Self::MAX_LEN).contains(&len));
        TailId {
            value: uuid.as_u128() & Self::mask(len),
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
        // Note: `Uuid::as_u128` packs the rightmost bytes of the UUID into the LSBs,
        //       thus "read big-endian", which is what we need.
        (uuid.as_u128() & Self::mask(self.len)) == self.value
    }

    /// Returns a bitmask for the given number of hex digits.
    #[inline]
    fn mask(len: u8) -> u128 {
        if len == Self::MAX_LEN {
            u128::MAX
        } else {
            (1u128 << (len as u32 * 4)) - 1
        }
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
        let mut buf = [0u8; TailId::MAX_LEN as usize];
        let mut len = 0usize;

        for &b in bytes {
            if b == b'-' {
                continue;
            }
            if len >= TailId::MAX_LEN as usize {
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
    /// The input is empty after stripping dashes.
    #[error("expected 1-32 hex characters (UUID suffix), got empty input")]
    Empty,

    /// The input exceeds 32 hex characters.
    #[error("expected 1-32 hex characters (UUID suffix), input too long")]
    TooLong,

    /// The input contains a non-hex byte.
    #[error("expected hex digit (0-9, a-f), found 0x{0:02x}")]
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

#[cfg(feature = "serde")]
impl Serialize for TailId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for TailId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <&str>::deserialize(deserializer)?;
        s.parse().map_err(de::Error::custom)
    }
}

#[cfg(feature = "schemars")]
impl JsonSchema for TailId {
    fn schema_name() -> String {
        "TailId".to_owned()
    }

    fn json_schema(_: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        use schemars::schema::{InstanceType, SchemaObject, StringValidation};

        let mut schema = SchemaObject::default();
        schema.instance_type = Some(InstanceType::String.into());
        schema.string = Some(Box::new(StringValidation {
            pattern: Some("^[0-9a-fA-F-]{1,36}$".to_owned()),
            ..Default::default()
        }));
        schema.metadata().description = Some("UUID suffix (1-32 hex characters)".to_owned());
        schema.into()
    }
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
    fn parse_normalizes() {
        let lower: TailId = "abcd".parse().unwrap();
        let upper: TailId = "ABCD".parse().unwrap();
        let dashes: TailId = "ab-cd".parse().unwrap();
        assert_eq!(lower, upper);
        assert_eq!(lower, dashes);
    }

    #[test]
    fn parse_rejects_invalid() {
        assert!(matches!(TailId::try_from(""), Err(ParseError::Empty)));
        assert!(matches!(TailId::try_from("---"), Err(ParseError::Empty)));
        assert!(matches!(
            TailId::try_from("0123456789abcdef0123456789abcdef0"),
            Err(ParseError::TooLong)
        ));
        assert!(matches!(
            TailId::try_from("ghij"),
            Err(ParseError::InvalidByte(b'g'))
        ));
    }

    #[test]
    fn display() {
        let tail: TailId = "3f6a4e7".parse().unwrap();
        assert_eq!(format!("{tail}"), "3f6a4e7");

        let tail: TailId = "00abcd".parse().unwrap();
        assert_eq!(format!("{tail}"), "00abcd");
    }

    #[test]
    fn matches_suffix() {
        let uuid = Uuid::parse_str("01234567-89ab-7def-8000-aabbccddeeff").unwrap();
        assert!(TailId::try_from("eeff").unwrap().matches(&uuid));
        assert!(
            TailId::try_from("0123456789ab7def8000aabbccddeeff")
                .unwrap()
                .matches(&uuid)
        );
        assert!(!TailId::try_from("ffff").unwrap().matches(&uuid));
    }

    #[test]
    fn resolve() {
        let id1 = Uuid::parse_str("01234567-89ab-7def-8000-000011112222").unwrap();
        let id2 = Uuid::parse_str("fedcba98-7654-7321-8000-000033332222").unwrap();
        let ids = vec![id1, id2];

        // Unique match
        assert_eq!(resolve_tail_id(&ids, &"11112222".parse().unwrap()), Ok(id1));
        assert_eq!(resolve_tail_id(&ids, &"33332222".parse().unwrap()), Ok(id2));

        // Not found
        assert!(matches!(
            resolve_tail_id(&ids, &"ffff".parse().unwrap()),
            Err(ResolveError::NotFound)
        ));

        // Ambiguous (both end in 2222)
        let result = resolve_tail_id(&ids, &"2222".parse().unwrap());
        assert!(matches!(result, Err(ResolveError::Ambiguous(v)) if v.len() == 2));
    }
}
