# uuid-suffix

Resolve UUIDs by their suffix.

Similar to how git lets you reference commits by a prefix of their hash, this crate lets you reference UUIDs by a suffix. Suffix matching works well for UUID versions with high entropy in their trailing bits (v4, v7), where a 7-character suffix uniquely identifies one UUID among ~268 million with high probability.

Note: UUID v1 and v6 embed a MAC address in the last 48 bits, so UUIDs from the same machine share suffixes. For these, you'll need longer suffixes.

```rust
use uuid_suffix::{UuidSuffix, resolve_uuid_suffix};
use uuid::Uuid;

let ids: Vec<Uuid> = vec![
    "019726fd-dc81-7b19-a27b-e8256d3f6a4e".parse().unwrap(),
    "fedcba98-7654-3210-8000-aabbccddeeff".parse().unwrap(),
];

// Find the UUID ending in "6a4e"
let suffix: UuidSuffix = "6a4e".parse().unwrap();
let found = resolve_uuid_suffix(&ids, &suffix).unwrap();
assert_eq!(found, ids[0]);

// Use matches() for direct comparison
let suffix: UuidSuffix = "eeff".parse().unwrap();
assert!(suffix.matches(&ids[1]));

// Create a suffix from a UUID for display
println!("{}", UuidSuffix::new(&ids[0]));  // "d3f6a4e"
```

## Full UUIDs

`UuidSuffix` accepts 1-32 hex characters, so it handles full UUIDs too. No need for `enum { Full(Uuid), Suffix(UuidSuffix) }`. Use `is_full()` and `to_uuid()` when you need to distinguish:

```rust
use uuid_suffix::UuidSuffix;

let full: UuidSuffix = "019726fd-dc81-7b19-a27b-e8256d3f6a4e".parse().unwrap();
assert!(full.is_full());
assert!(full.to_uuid().is_some());
```

## Features

- **serde**: Serialization support. Always uses string representation (even in binary formats) because suffixes can be odd-length hex (nibbles can't be losslessly encoded as bytes).
- **schemars**: JSON Schema support for use with MCP servers and similar.

## Related work

If you just want a shorter way to encode UUIDs, [`short-uuid`](https://docs.rs/short-uuid) might fit the bill as well.
