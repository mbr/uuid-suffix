#![doc = include_str!("../README.md")]

use std::{fmt, str::FromStr};

use thiserror::Error;
use uuid::Uuid;

/// A time-ordered UUID (v6 or v7) with convenient short display.
///
/// Displays as the last 7 hex characters by default. Use width specifier for more (e.g.,
/// `{:9}`), or `{:#}` for the full hyphenated UUID.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
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

// --- Serialization implementations ---

#[cfg(feature = "serde")]
mod serde_impl {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::*;

    impl Serialize for Dandruff {
        /// Serializes the UUID.
        ///
        /// Uses hyphenated string format for human-readable serializers (JSON, TOML) and raw
        /// 16-byte array for binary serializers (bincode, msgpack).
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            if serializer.is_human_readable() {
                serializer.serialize_str(&self.0.as_hyphenated().to_string())
            } else {
                self.0.as_bytes().serialize(serializer)
            }
        }
    }

    impl<'de> Deserialize<'de> for Dandruff {
        /// Deserializes a UUID.
        ///
        /// Accepts hyphenated string format from human-readable deserializers and raw 16-byte
        /// array from binary deserializers. Does not validate UUID version.
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            if deserializer.is_human_readable() {
                let s = String::deserialize(deserializer)?;
                let uuid =
                    Uuid::parse_str(&s).map_err(|e| serde::de::Error::custom(e.to_string()))?;
                Ok(Dandruff::from_uuid_unchecked(uuid))
            } else {
                let bytes = <[u8; 16]>::deserialize(deserializer)?;
                Ok(Dandruff::from_uuid_unchecked(Uuid::from_bytes(bytes)))
            }
        }
    }
}

#[cfg(feature = "borsh")]
mod borsh_impl {
    use borsh::{BorshDeserialize, BorshSerialize};

    use super::*;

    impl BorshSerialize for Dandruff {
        /// Serializes the UUID as 16 raw bytes.
        fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
            self.0.serialize(writer)
        }
    }

    impl BorshDeserialize for Dandruff {
        /// Deserializes a UUID from 16 raw bytes.
        ///
        /// Does not validate UUID version.
        fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
            let uuid = Uuid::deserialize_reader(reader)?;
            Ok(Dandruff::from_uuid_unchecked(uuid))
        }
    }
}

#[cfg(feature = "bytemuck")]
mod bytemuck_impl {
    use super::*;

    // SAFETY: Dandruff is a transparent wrapper around Uuid, which is Pod.
    // The repr is not explicitly transparent, but Dandruff(Uuid) has the same layout as Uuid.
    unsafe impl bytemuck::Pod for Dandruff {}

    // SAFETY: Dandruff is a transparent wrapper around Uuid, which is Zeroable.
    unsafe impl bytemuck::Zeroable for Dandruff {}
}

#[cfg(feature = "arbitrary")]
mod arbitrary_impl {
    use arbitrary::Arbitrary;

    use super::*;

    impl<'a> Arbitrary<'a> for Dandruff {
        /// Generates an arbitrary [`Dandruff`] by wrapping an arbitrary UUID.
        ///
        /// Note: The generated UUID may not be a valid v6 or v7 UUID.
        fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
            let uuid = Uuid::arbitrary(u)?;
            Ok(Dandruff::from_uuid_unchecked(uuid))
        }
    }
}

// --- Database integrations ---

#[cfg(feature = "sqlx-postgres")]
mod sqlx_postgres_impl {
    use sqlx::{
        Decode, Encode, Postgres, Type,
        postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo, PgValueRef},
    };

    use super::*;

    impl Type<Postgres> for Dandruff {
        fn type_info() -> PgTypeInfo {
            <Uuid as Type<Postgres>>::type_info()
        }

        fn compatible(ty: &PgTypeInfo) -> bool {
            <Uuid as Type<Postgres>>::compatible(ty)
        }
    }

    impl PgHasArrayType for Dandruff {
        fn array_type_info() -> PgTypeInfo {
            <Uuid as PgHasArrayType>::array_type_info()
        }
    }

    impl Encode<'_, Postgres> for Dandruff {
        fn encode_by_ref(
            &self,
            buf: &mut PgArgumentBuffer,
        ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
            <Uuid as Encode<'_, Postgres>>::encode_by_ref(&self.0, buf)
        }
    }

    impl Decode<'_, Postgres> for Dandruff {
        fn decode(value: PgValueRef<'_>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            let uuid = <Uuid as Decode<'_, Postgres>>::decode(value)?;
            Ok(Dandruff::from_uuid_unchecked(uuid))
        }
    }
}

#[cfg(feature = "sqlx-sqlite")]
mod sqlx_sqlite_impl {
    use sqlx::{
        Decode, Encode, Sqlite, Type,
        sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef},
    };

    use super::*;

    impl Type<Sqlite> for Dandruff {
        fn type_info() -> SqliteTypeInfo {
            <Uuid as Type<Sqlite>>::type_info()
        }

        fn compatible(ty: &SqliteTypeInfo) -> bool {
            <Uuid as Type<Sqlite>>::compatible(ty)
        }
    }

    impl<'q> Encode<'q, Sqlite> for Dandruff {
        fn encode_by_ref(
            &self,
            args: &mut Vec<SqliteArgumentValue<'q>>,
        ) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
            <Uuid as Encode<'q, Sqlite>>::encode_by_ref(&self.0, args)
        }
    }

    impl Decode<'_, Sqlite> for Dandruff {
        fn decode(
            value: SqliteValueRef<'_>,
        ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
            let uuid = <Uuid as Decode<'_, Sqlite>>::decode(value)?;
            Ok(Dandruff::from_uuid_unchecked(uuid))
        }
    }
}

#[cfg(feature = "datafusion")]
mod datafusion_impl {
    use arrow_schema::DataType;

    use super::*;

    impl Dandruff {
        /// The Arrow data type for [`Dandruff`] values.
        ///
        /// Returns `FixedSizeBinary(16)` for efficient storage of UUIDs.
        pub const ARROW_DATA_TYPE: DataType = DataType::FixedSizeBinary(16);

        /// Converts to Arrow-compatible bytes.
        #[inline]
        pub fn to_arrow_bytes(&self) -> [u8; 16] {
            *self.0.as_bytes()
        }

        /// Creates a [`Dandruff`] from Arrow bytes.
        ///
        /// Does not validate UUID version.
        #[inline]
        pub fn from_arrow_bytes(bytes: [u8; 16]) -> Self {
            Self::from_uuid_unchecked(Uuid::from_bytes(bytes))
        }
    }
}

// --- Timestamp extraction ---

#[cfg(feature = "chrono")]
mod chrono_impl {
    use chrono::{DateTime, TimeZone, Utc};

    use super::*;

    impl Dandruff {
        /// Extracts the timestamp from this UUID as a [`chrono::DateTime<Utc>`].
        ///
        /// Returns `None` if the UUID does not contain a valid timestamp (non-v6/v7).
        pub fn chrono_datetime(&self) -> Option<DateTime<Utc>> {
            let ts = self.0.get_timestamp()?;
            let (secs, nanos) = ts.to_unix();
            Utc.timestamp_opt(secs as i64, nanos).single()
        }
    }
}

#[cfg(feature = "jiff")]
mod jiff_impl {
    use jiff::Timestamp;

    use super::*;

    impl Dandruff {
        /// Extracts the timestamp from this UUID as a [`jiff::Timestamp`].
        ///
        /// Returns `None` if the UUID does not contain a valid timestamp (non-v6/v7).
        pub fn jiff_timestamp(&self) -> Option<Timestamp> {
            let ts = self.0.get_timestamp()?;
            let (secs, nanos) = ts.to_unix();
            Timestamp::new(secs as i64, nanos as i32).ok()
        }
    }
}

// --- CLI and schema integrations ---

#[cfg(feature = "clap")]
mod clap_impl {
    use std::ffi::OsStr;

    use clap::builder::{StringValueParser, TypedValueParser, ValueParserFactory};

    use super::*;

    /// Value parser for [`Dandruff`] that accepts full UUID strings.
    #[derive(Clone, Debug)]
    pub struct DandruffValueParser;

    impl TypedValueParser for DandruffValueParser {
        type Value = Dandruff;

        fn parse_ref(
            &self,
            cmd: &clap::Command,
            arg: Option<&clap::Arg>,
            value: &OsStr,
        ) -> Result<Self::Value, clap::Error> {
            let s = StringValueParser::new().parse_ref(cmd, arg, value)?;
            s.parse::<Dandruff>().map_err(|e| {
                clap::Error::raw(clap::error::ErrorKind::InvalidValue, format!("{e}\n"))
            })
        }
    }

    impl ValueParserFactory for Dandruff {
        type Parser = DandruffValueParser;

        fn value_parser() -> Self::Parser {
            DandruffValueParser
        }
    }
}

#[cfg(feature = "proptest")]
mod proptest_impl {
    use proptest::{
        arbitrary::Arbitrary,
        strategy::{BoxedStrategy, Strategy},
    };

    use super::*;

    impl Arbitrary for Dandruff {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        /// Generates arbitrary [`Dandruff`] values by wrapping arbitrary UUIDs.
        ///
        /// Note: The generated UUID may not be a valid v6 or v7 UUID.
        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            proptest::array::uniform16(proptest::num::u8::ANY)
                .prop_map(|bytes| Dandruff::from_uuid_unchecked(Uuid::from_bytes(bytes)))
                .boxed()
        }
    }
}

#[cfg(feature = "schemars")]
mod schemars_impl {
    use schemars::{
        JsonSchema,
        r#gen::SchemaGenerator,
        schema::{InstanceType, Schema, SchemaObject, StringValidation},
    };

    use super::*;

    impl JsonSchema for Dandruff {
        fn schema_name() -> String {
            "Dandruff".to_string()
        }

        fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
            Schema::Object(SchemaObject {
                instance_type: Some(InstanceType::String.into()),
                format: Some("uuid".to_string()),
                string: Some(Box::new(StringValidation {
                    pattern: Some(
                        "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[67][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$"
                            .to_string(),
                    ),
                    ..Default::default()
                })),
                ..Default::default()
            })
        }
    }
}

/// A parsed short ID for efficient suffix matching against UUIDs.
///
/// Stores the short ID as a `u128` value with a length field, enabling fast bitwise comparison.
/// Accepts 1-32 lowercase hex characters (dashes and spaces are stripped during parsing,
/// uppercase is normalized to lowercase).
///
/// # Example
///
/// ```
/// use dandruff::ShortId;
///
/// let short: ShortId = "3f6a4e7".parse().unwrap();
/// assert_eq!(format!("{}", short), "3f6a4e7");
/// ```
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ShortId {
    /// The short ID value, right-aligned (least significant bits).
    value: u128,
    /// Number of hex digits (1-32).
    len: u8,
}

impl ShortId {
    /// Minimum number of hex characters required.
    pub const MIN_LEN: u8 = 1;
    /// Maximum number of hex characters allowed (full UUID).
    pub const MAX_LEN: u8 = 32;

    /// Returns the number of hex digits in this short ID.
    #[inline]
    pub fn len(&self) -> u8 {
        self.len
    }

    /// Returns `true` if this short ID has zero length.
    ///
    /// Note: A zero-length ShortId cannot be constructed via `TryFrom`, so this always returns
    /// `false` for validly constructed instances.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the underlying value as a `u128`.
    ///
    /// The value is right-aligned: for a 7-character short ID "3f6a4e7", the value is `0x3f6a4e7`.
    #[inline]
    pub fn as_u128(&self) -> u128 {
        self.value
    }

    /// Checks if this short ID matches the suffix of the given [`Dandruff`].
    ///
    /// Comparison is performed via masked `u128` bitwise operations.
    #[inline]
    pub fn matches(&self, dandruff: &Dandruff) -> bool {
        let mask = if self.len == 32 {
            u128::MAX
        } else {
            (1u128 << (self.len as u32 * 4)) - 1
        };
        let uuid_bits = dandruff.0.as_u128();
        (uuid_bits & mask) == self.value
    }
}

impl fmt::Display for ShortId {
    /// Formats the short ID as lowercase hex.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.len == 0 {
            return Ok(());
        }
        // Format value as hex, then take the last `len` characters
        // We need to handle leading zeros correctly
        write!(f, "{:0>width$x}", self.value, width = self.len as usize)
    }
}

impl FromStr for ShortId {
    type Err = ShortIdError;

    /// Parses a [`ShortId`] from a string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl TryFrom<&str> for ShortId {
    type Error = ShortIdError;

    /// Parses a short ID from a string.
    ///
    /// Normalizes input by stripping dashes and spaces, and converting to lowercase.
    /// Validates that the result is 1-32 lowercase hex characters.
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        // Normalize: strip dashes/spaces, lowercase
        let mut normalized = [0u8; 32];
        let mut len = 0usize;

        for c in s.chars() {
            if c == '-' || c.is_whitespace() {
                continue;
            }
            if len >= 32 {
                return Err(ShortIdError::TooLong);
            }
            let hex_digit = match c {
                '0'..='9' => c as u8 - b'0',
                'a'..='f' => c as u8 - b'a' + 10,
                'A'..='F' => c as u8 - b'A' + 10,
                _ => return Err(ShortIdError::InvalidCharacter(c)),
            };
            normalized[len] = hex_digit;
            len += 1;
        }

        if len == 0 {
            return Err(ShortIdError::Empty);
        }

        // Build the u128 value from hex digits
        let mut value = 0u128;
        for &digit in &normalized[..len] {
            value = (value << 4) | (digit as u128);
        }

        Ok(ShortId {
            value,
            len: len as u8,
        })
    }
}

impl TryFrom<&[u8]> for ShortId {
    type Error = ShortIdError;

    /// Parses a short ID from a byte slice.
    ///
    /// Normalizes input by stripping dashes and spaces, and converting to lowercase.
    /// Validates that the result is 1-32 lowercase hex characters.
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        // Normalize: strip dashes/spaces, lowercase
        let mut normalized = [0u8; 32];
        let mut len = 0usize;

        for &b in bytes {
            if b == b'-' || b.is_ascii_whitespace() {
                continue;
            }
            if len >= 32 {
                return Err(ShortIdError::TooLong);
            }
            let hex_digit = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(ShortIdError::InvalidCharacter(b as char)),
            };
            normalized[len] = hex_digit;
            len += 1;
        }

        if len == 0 {
            return Err(ShortIdError::Empty);
        }

        // Build the u128 value from hex digits
        let mut value = 0u128;
        for &digit in &normalized[..len] {
            value = (value << 4) | (digit as u128);
        }

        Ok(ShortId {
            value,
            len: len as u8,
        })
    }
}

/// Error returned when parsing a [`ShortId`].
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ShortIdError {
    /// The input is empty after stripping dashes and spaces.
    #[error("short ID cannot be empty")]
    Empty,

    /// The input exceeds 32 hex characters.
    #[error("short ID cannot exceed 32 hex characters")]
    TooLong,

    /// The input contains a non-hex character.
    #[error("invalid character in short ID: '{0}'")]
    InvalidCharacter(char),
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
    /// The pattern could not be parsed as a [`ShortId`].
    #[error("invalid short ID: {0}")]
    InvalidShortId(#[source] ShortIdError),

    /// No ID matched the pattern.
    #[error("no ID matched the pattern")]
    NotFound,

    /// Multiple IDs matched the pattern.
    #[error("pattern is ambiguous, matched {} IDs", .0.len())]
    Ambiguous(Vec<Dandruff>),
}

/// Resolves a [`ShortId`] against a collection of IDs using suffix matching.
///
/// # Errors
///
/// Returns [`ResolveError::NotFound`] if no ID matches.
/// Returns [`ResolveError::Ambiguous`] if multiple IDs match, containing all matches.
pub fn resolve_short_id<'a, I>(iter: I, short_id: &ShortId) -> Result<Dandruff, ResolveError>
where
    I: IntoIterator<Item = &'a Dandruff>,
{
    let matches: Vec<Dandruff> = iter
        .into_iter()
        .filter(|id| short_id.matches(id))
        .copied()
        .collect();

    match matches.len() {
        0 => Err(ResolveError::NotFound),
        1 => Ok(matches[0]),
        _ => Err(ResolveError::Ambiguous(matches)),
    }
}

/// Resolves a short ID pattern against a collection of IDs using suffix matching.
///
/// Parses the pattern as a [`ShortId`] (normalizing dashes, spaces, and case) and matches
/// against the suffix of each UUID's hex representation.
///
/// # Errors
///
/// Returns [`ResolveError::InvalidShortId`] if the pattern cannot be parsed.
/// Returns [`ResolveError::NotFound`] if no ID matches.
/// Returns [`ResolveError::Ambiguous`] if multiple IDs match, containing all matches.
pub fn resolve<'a, I>(iter: I, pattern: &str) -> Result<Dandruff, ResolveError>
where
    I: IntoIterator<Item = &'a Dandruff>,
{
    let short_id = ShortId::try_from(pattern).map_err(ResolveError::InvalidShortId)?;
    resolve_short_id(iter, &short_id)
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

        // Empty pattern
        assert!(matches!(
            resolve(&ids, ""),
            Err(ResolveError::InvalidShortId(ShortIdError::Empty))
        ));
        // Invalid hex characters
        assert!(matches!(
            resolve(&ids, "ghij1234"),
            Err(ResolveError::InvalidShortId(
                ShortIdError::InvalidCharacter('g')
            ))
        ));
    }

    #[test]
    fn resolve_normalizes_input() {
        let id = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("01234567-89ab-7def-8000-aabbccddeeff").expect("valid uuid"),
        );
        let ids = vec![id];

        // Suffix matching with normalization
        assert_eq!(resolve(&ids, "AABBCCDDEEFF"), Ok(id));
        assert_eq!(resolve(&ids, "CCDD-EEFF"), Ok(id));
        assert_eq!(resolve(&ids, "ddee ff"), Ok(id));
    }

    // ShortId tests

    #[test]
    fn short_id_parse_valid() {
        let short: ShortId = "3f6a4e7".parse().expect("valid short id");
        assert_eq!(short.len(), 7);
        assert_eq!(short.as_u128(), 0x3f6a4e7);
    }

    #[test]
    fn short_id_parse_normalizes() {
        let lower: ShortId = "abcd".parse().expect("valid");
        let upper: ShortId = "ABCD".parse().expect("valid");
        let dashes: ShortId = "ab-cd".parse().expect("valid");
        let spaces: ShortId = "ab cd".parse().expect("valid");

        assert_eq!(lower, upper);
        assert_eq!(lower, dashes);
        assert_eq!(lower, spaces);
    }

    #[test]
    fn short_id_parse_rejects_empty() {
        let result = ShortId::try_from("");
        assert!(matches!(result, Err(ShortIdError::Empty)));

        // Only whitespace/dashes
        let result = ShortId::try_from("  - - ");
        assert!(matches!(result, Err(ShortIdError::Empty)));
    }

    #[test]
    fn short_id_parse_rejects_too_long() {
        let result = ShortId::try_from("0123456789abcdef0123456789abcdef0");
        assert!(matches!(result, Err(ShortIdError::TooLong)));
    }

    #[test]
    fn short_id_parse_rejects_invalid_chars() {
        let result = ShortId::try_from("ghij");
        assert!(matches!(result, Err(ShortIdError::InvalidCharacter('g'))));
    }

    #[test]
    fn short_id_display_roundtrip() {
        let original = "3f6a4e7";
        let short: ShortId = original.parse().expect("valid");
        assert_eq!(format!("{}", short), original);
    }

    #[test]
    fn short_id_display_preserves_leading_zeros() {
        let short: ShortId = "00abcd".parse().expect("valid");
        assert_eq!(format!("{}", short), "00abcd");
    }

    #[test]
    fn short_id_matches_suffix() {
        let id = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("01234567-89ab-7def-8000-aabbccddeeff").expect("valid uuid"),
        );

        let suffix: ShortId = "eeff".parse().expect("valid");
        assert!(suffix.matches(&id));

        let full: ShortId = "0123456789ab7def8000aabbccddeeff".parse().expect("valid");
        assert!(full.matches(&id));

        let wrong: ShortId = "ffff".parse().expect("valid");
        assert!(!wrong.matches(&id));
    }

    #[test]
    fn short_id_from_bytes() {
        let short = ShortId::try_from(b"3f6a4e7".as_slice()).expect("valid");
        assert_eq!(short.len(), 7);
        assert_eq!(format!("{}", short), "3f6a4e7");
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

#[cfg(all(test, feature = "serde", feature = "v7"))]
mod serde_tests {
    use super::*;

    #[test]
    fn serde_json_roundtrip() {
        let id = Dandruff::new_v7();
        let json = serde_json::to_string(&id).expect("serialize");
        let parsed: Dandruff = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, parsed);
    }

    #[test]
    fn serde_json_format() {
        let id = Dandruff::from_uuid_unchecked(
            Uuid::parse_str("019726fd-dc81-7b19-a27b-e8256d3f6a4e").expect("valid uuid"),
        );
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"019726fd-dc81-7b19-a27b-e8256d3f6a4e\"");
    }

    #[test]
    fn serde_bincode_roundtrip() {
        let id = Dandruff::new_v7();
        let bytes = bincode::serialize(&id).expect("serialize");
        assert_eq!(bytes.len(), 16);
        let parsed: Dandruff = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(id, parsed);
    }
}

#[cfg(all(test, feature = "borsh", feature = "v7"))]
mod borsh_tests {
    use borsh::{BorshDeserialize, BorshSerialize};

    use super::*;

    #[test]
    fn borsh_roundtrip() {
        let id = Dandruff::new_v7();
        let mut bytes = Vec::new();
        id.serialize(&mut bytes).expect("serialize");
        assert_eq!(bytes.len(), 16);
        let parsed = Dandruff::deserialize(&mut bytes.as_slice()).expect("deserialize");
        assert_eq!(id, parsed);
    }
}

#[cfg(all(test, feature = "datafusion", feature = "v7"))]
mod datafusion_tests {
    use arrow_schema::DataType;

    use super::*;

    #[test]
    fn arrow_data_type() {
        assert_eq!(Dandruff::ARROW_DATA_TYPE, DataType::FixedSizeBinary(16));
    }

    #[test]
    fn arrow_bytes_roundtrip() {
        let id = Dandruff::new_v7();
        let bytes = id.to_arrow_bytes();
        let restored = Dandruff::from_arrow_bytes(bytes);
        assert_eq!(id, restored);
    }
}

#[cfg(all(test, feature = "chrono", feature = "v7"))]
mod chrono_tests {
    use super::*;

    #[test]
    fn chrono_datetime_extracts_timestamp() {
        let id = Dandruff::new_v7();
        let dt = id.chrono_datetime();
        assert!(dt.is_some());
        let dt = dt.expect("should have timestamp");
        let now = chrono::Utc::now();
        let diff = (now - dt).num_seconds().abs();
        assert!(diff < 2, "timestamp should be within 2 seconds of now");
    }

    #[test]
    fn chrono_datetime_returns_none_for_non_time_uuid() {
        let nil = Dandruff::from_uuid_unchecked(uuid::Uuid::nil());
        assert!(nil.chrono_datetime().is_none());
    }
}

#[cfg(all(test, feature = "jiff", feature = "v7"))]
mod jiff_tests {
    use super::*;

    #[test]
    fn jiff_timestamp_extracts_timestamp() {
        let id = Dandruff::new_v7();
        let ts = id.jiff_timestamp();
        assert!(ts.is_some());
        let ts = ts.expect("should have timestamp");
        let now = jiff::Timestamp::now();
        let diff = (now - ts).get_seconds().abs();
        assert!(diff < 2, "timestamp should be within 2 seconds of now");
    }

    #[test]
    fn jiff_timestamp_returns_none_for_non_time_uuid() {
        let nil = Dandruff::from_uuid_unchecked(uuid::Uuid::nil());
        assert!(nil.jiff_timestamp().is_none());
    }
}

#[cfg(all(test, feature = "schemars"))]
mod schemars_tests {
    use schemars::JsonSchema;

    use super::*;

    #[test]
    fn json_schema_name() {
        assert_eq!(Dandruff::schema_name(), "Dandruff");
    }

    #[test]
    fn json_schema_has_uuid_format() {
        let schema = schemars::schema_for!(Dandruff);
        let json = serde_json::to_string(&schema).expect("serialize schema");
        assert!(json.contains("\"format\":\"uuid\""));
    }
}
