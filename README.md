# dandruff

Got an itch to use snowflake-style IDs? Dandruff provides a UUIDv7 newtype that scratches it.

## What are snowflake IDs?

Twitter introduced Snowflake IDs in 2010: 64-bit identifiers encoding a timestamp, machine ID, and sequence number. They're time-ordered, globally unique without coordination, and fit in a single integer. Many systems adopted similar schemes (Discord, Instagram, etc.).

## Why UUIDv7 instead?

UUIDv7 (RFC 9562) brings the same benefits to the UUID format:

- **Time-ordered**: Embeds a millisecond Unix timestamp, so IDs sort chronologically
- **Globally unique**: 74 random bits make collisions astronomically unlikely
- **Database-friendly**: Native UUID support in PostgreSQL, no special columns needed
- **Standard format**: 128-bit UUID, works with existing tooling

The main difference: UUIDv7 is 128 bits vs Snowflake's 64 bits, trading compactness for simplicity (no machine ID coordination required).

## What dandruff adds

The `uuid` crate handles generation. Dandruff wraps it with:

- **Short ID display**: `Display` shows just the last 8 hex chars (e.g., `a1b2c3d4`)
- **Full ID access**: Use `{:#}` for the complete hyphenated UUID
- **Short ID resolution**: Match partial IDs against a collection (like git's short commit hashes)
- **Type safety**: `TryFrom<Uuid>` validates v7, preventing accidental v4 usage
- **Integrations**: Feature-gated support for serde, sqlx, DataFusion, and more

## Usage

```rust
use dandruff::{Dandruff, resolve};

// Generate a new ID
let id = Dandruff::new();
println!("{}", id);      // "a1b2c3d4" (short)
println!("{:#}", id);    // "01234567-89ab-7cde-8f01-23456789a1b2" (full)

// Parse from string
let parsed: Dandruff = "01234567-89ab-7cde-8f01-23456789a1b2".parse()?;

// Resolve short ID against a collection
let ids: Vec<Dandruff> = load_from_database();
let matched = resolve(&ids, "a1b2")?;
```

## Features

- `serde`: Serialize/deserialize (efficient binary format for non-human-readable)
- `sqlx-postgres`: PostgreSQL integration via sqlx
- `sqlx-sqlite`: SQLite integration via sqlx
- `datafusion`: Arrow/DataFusion integration
- `jiff`: Timestamp extraction as `jiff::Timestamp`
- `clap`: CLI argument parsing
- `fast-rng`, `rng-rand`, `rng-getrandom`, `js`: Passed through to `uuid` crate
