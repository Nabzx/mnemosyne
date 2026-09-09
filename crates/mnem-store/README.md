# mnem-store

The core of [Mnemosyne](https://github.com/Nabzx/mnemosyne): version control for
AI agent memory.

This crate holds the object model, the canonical CBOR encoding, the
content-addressed object store (`redb`), the commit graph, and every
deterministic operation on them: `commit`, `branch`, `checkout`, time travel,
the three-way `merge`, `blame`, and `bisect`. It never opens a network
connection and never calls a model.

The `mnem` command-line tool is the [`mnem-git`](https://crates.io/crates/mnem-git)
crate. The Python SDK is `mnem-agents` on PyPI.

```rust
use mnem_store::Store;

let store = Store::init("./agent-memory")?;
store.stage(&node)?;
let commit = store.commit("learn the plan tier", "support-agent", now_ms)?;
let blame = store.blame("customer-4821", None)?; // which commit set this, and why
```

Licence: Apache-2.0. On-disk format: see
[`docs/format/`](https://github.com/Nabzx/mnemosyne/tree/main/docs/format).
