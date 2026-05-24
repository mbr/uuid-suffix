# tail-id

Resolve UUIDs by their suffix, git-style.

## What it does

When you have a collection of UUIDs, you often want to reference them by a short suffix rather than typing all 32 hex characters. This is the same approach git uses for commit hashes.

```rust
use tail_id::{TailId, resolve};
use uuid::Uuid;

let ids: Vec<Uuid> = vec![
    "019726fd-dc81-7b19-a27b-e8256d3f6a4e".parse().unwrap(),
    "fedcba98-7654-3210-8000-aabbccddeeff".parse().unwrap(),
];

// Find the UUID ending in "6a4e"
let found = resolve(&ids, "6a4e").unwrap();
assert_eq!(found, ids[0]);

// Parse and reuse a tail ID for multiple lookups
let tail: TailId = "eeff".parse().unwrap();
assert!(tail.matches(&ids[1]));
```

## API

- `TailId` - Parsed tail ID for efficient matching (1-32 hex chars, case-insensitive, strips dashes/spaces)
- `resolve(iter, pattern)` - Find the unique UUID matching a suffix pattern
- `resolve_tail_id(iter, tail_id)` - Same, but with a pre-parsed `TailId`

Resolution returns `Err(ResolveError::Ambiguous)` if multiple UUIDs match, or `Err(ResolveError::NotFound)` if none do.

## Why suffix matching?

UUID versions like v7 (and v4) have high entropy in their trailing bits, making suffixes good discriminators. A 7-character suffix (28 bits) uniquely identifies one UUID among ~268 million with high probability.

Some UUID versions (v1, v6) embed a MAC address in the last 48 bits, which means UUIDs from the same machine share suffixes. For these, you'll need longer tail IDs or should match against the timestamp portion instead.
