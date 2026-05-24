# dandruff

Got an itch to use snowflake-style IDs? Dandruff provides a time-ordered UUID newtype (around [`uuid`](https://docs.rs/uuid)) that scratches it.

## What are snowflake IDs and UUID v6/7?

Twitter introduced [Snowflake IDs](https://en.wikipedia.org/wiki/Snowflake_ID) in 2010: 64-bit identifiers encoding a timestamp, machine ID, and sequence number. They're time-ordered, globally unique without coordination, and fit in a single integer. Many systems adopted similar schemes (Discord, Instagram, etc.).

UUIDv6 is the UUID equivalent of Snowflake: timestamp + node ID. UUIDv7 ([RFC 9562](https://www.rfc-editor.org/rfc/rfc9562.html)) is the lovechild of v6 and v4: it keeps v6's time-ordering but replaces the node ID with random bits like v4, eliminating the need for machine coordination.

```text
Version 6:

 1ef726fd-dc81-6b19-a27b-010203040506   (v6: timestamp + node ID)
 \_____________|_/  |\_/ \_________/
      |        |    |        |
  timestamp  ver:6  |    node ID (clock seq + MAC address)
                    |
           variant--+

Version 7:

 019726fd-dc81-7b19-a27b-e8256d3f6a4e   (v7: timestamp + random)
 \___________/ |\___________________/
      |        |    |         |
  timestamp  ver:7  |    74 random bits 
                    |   
           variant--+
```

## What dandruff adds

Dandruff wraps `uuid::Uuid` to provide:

- **Short IDs display**: `Display` shows just the last 7 hex chars (e.g., `a1b2c3d7`) by default. This short-ID style has been pioneered by [git](https://git-scm.com). `dandruff` short IDs differ in that they are taken from the end of the ID, not the beginning, since timestamp-based UUIDs have the same prefix at least 3 days.
- **Short ID resolution**: Match partial IDs against a collection.
- **Flexible formatting**: Use `{:#}` for the complete hyphenated UUID, or `{:9}` if you need longer IDs.
- **Type safety**: `TryFrom<Uuid>` validates v6/v7, preventing accidental v4 usage
- **Integrations**: Feature-gated support for [`serde`](https://serde.rs), [`sqlx`](https://docs.rs/sqlx), [DataFusion](https://docs.rs/datafusion), and more

## Usage

```rust,ignore
use dandruff::{Dandruff, resolve};

// Generate a new v7 ID (random, no coordination needed)
let id = Dandruff::new_v7();
println!("{}", id);      // "3f6a4e7" (short, 7 chars)
println!("{:#}", id);    // "019726fd-dc81-7b19-a27b-e8256d3f6a4e" (full)
println!("{:12}", id);   // "56d3f6a4e" (custom width)

// Or generate a v6 ID with explicit node ID (like Snowflake)
let node_id = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
let id_v6 = Dandruff::new_v6(&node_id);

// Parse from string (must be valid v6 or v7)
let parsed: Dandruff = "019726fd-dc81-7b19-a27b-e8256d3f6a4e".parse().unwrap();

// Resolve short ID against a collection
let ids = vec![Dandruff::new_v7(), Dandruff::new_v7(), Dandruff::new_v7()];
let target = ids[1];
let matched = resolve(&ids, &format!("{}", target)).unwrap();
assert_eq!(matched, target);
```

## Features

**Version support (both default):**
- `v6`: Enables `new_v6()` for node-based IDs
- `v7`: Enables `new_v7()` for random-based IDs

**Serialization:**
- `serde`: Serialize/deserialize (efficient binary format for non-human-readable)
- `borsh`: Borsh serialization
- `bytemuck`: Pod/Zeroable traits
- `sqlx-postgres`: PostgreSQL integration via sqlx
- `sqlx-sqlite`: SQLite integration via sqlx
- `datafusion`: Arrow/DataFusion integration

**Utilities:**
- `chrono`: Timestamp extraction as `chrono::DateTime<Utc>`
- `jiff`: Timestamp extraction as `jiff::Timestamp`
- `clap`: CLI argument parsing
- `proptest`: Property-based testing support
- `schemars`: JSON Schema generation

**Passed through to `uuid`:**
- `arbitrary`, `fast-rng`, `rng-rand`, `rng-getrandom`, `js`
