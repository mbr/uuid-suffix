# tail-id

Resolve UUIDs by their suffix.

Similar to how git lets you reference commits by a prefix of their hash, this crate lets you reference UUIDs by a suffix (tail). Suffix matching works well for UUID versions with high entropy in their trailing bits (v4, v7), where a 7-character tail uniquely identifies one UUID among ~268 million with high probability.

Note: UUID v1 and v6 embed a MAC address in the last 48 bits, so UUIDs from the same machine share suffixes. For these, you'll need longer tail IDs.

```rust
use tail_id::{TailId, resolve_tail_id};
use uuid::Uuid;

let ids: Vec<Uuid> = vec![
    "019726fd-dc81-7b19-a27b-e8256d3f6a4e".parse().unwrap(),
    "fedcba98-7654-3210-8000-aabbccddeeff".parse().unwrap(),
];

// Find the UUID ending in "6a4e"
let tail: TailId = "6a4e".parse().unwrap();
let found = resolve_tail_id(&ids, &tail).unwrap();
assert_eq!(found, ids[0]);

// Use matches() for direct comparison
let tail: TailId = "eeff".parse().unwrap();
assert!(tail.matches(&ids[1]));

// Create a tail ID from a UUID for display
println!("{}", TailId::new(&ids[0]));  // "d3f6a4e"
```

## Features

- **serde**: `serde` serialization support. Always uses string representation (even in binary formats) because tail IDs can be odd-length hex (nibbles can't be losslessly encoded as bytes).
- **schemars**: JSON Schema support for use with MCP servers and similar.
